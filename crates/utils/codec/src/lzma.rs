//! https://github.com/Shelwien/lzma_sh/blob/master/LzmaSpec.cpp

use std::{
    collections::VecDeque,
    io::{Read, Write},
};
use util_general::ReadExt as _;

const K_TOP_VALUE: u32 = 1 << 24;
const K_NUM_BIT_MODEL_TOTAL_BITS: u32 = 11;
const K_NUM_MOVE_BITS: u32 = 5;
const PROB_INIT_VAL: u16 = (1 << K_NUM_BIT_MODEL_TOTAL_BITS) / 2;
const K_NUM_POS_BITS_MAX: u32 = 4;
const LZMA_DICT_MIN: u32 = 1 << 12;
const K_NUM_LEN_TO_POS_STATES: u32 = 4;
const K_NUM_ALIGN_BITS: usize = 4;
const K_NUM_ALIGN_BITS_COUNT: usize = 1 << K_NUM_ALIGN_BITS;
const K_END_POS_MODEL_INDEX: usize = 14;
const K_NUM_FULL_DISTANCES: usize = 1 << (K_END_POS_MODEL_INDEX >> 1);
const K_NUM_STATES: usize = 12;
const K_MATCH_MIN_LEN: u32 = 2;

#[derive(Debug, Default, Clone)]
struct RangeDecoder {
    range: u32,
    code: u32,
}

impl RangeDecoder {
    fn init<R: Read>(&mut self, mut reader: R) -> std::io::Result<()> {
        let byte = u8::from_le_bytes(reader.read_const()?);
        self.range = 0xFFFFFFFF;
        self.code = u32::from_be_bytes(reader.read_const()?);
        if byte != 0 || self.code == self.range {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "LZMA failed to initialize RangeDecoder",
            ));
        }
        Ok(())
    }

    fn is_finished(&self) -> bool {
        self.code == 0
    }

    fn normalize<R: Read>(&mut self, mut reader: R) -> std::io::Result<()> {
        if self.range < K_TOP_VALUE {
            self.range <<= 8;
            self.code <<= 8;
            self.code |= u8::from_le_bytes(reader.read_const()?) as u32;
        }
        Ok(())
    }

    fn decode_direct_bits<R: Read>(
        &mut self,
        mut reader: R,
        num_bits: u32,
    ) -> std::io::Result<u32> {
        let mut res: u32 = 0;
        for _ in 0..num_bits {
            self.range >>= 1;
            self.code = self.code.wrapping_sub(self.range);
            let t = (0u32).wrapping_sub(self.code >> 31);
            self.code = self.code.wrapping_add(self.range & t);

            if self.code == self.range {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Range decoder invalid state",
                ));
            }

            self.normalize(&mut reader)?;

            res <<= 1;
            res += t.wrapping_add(1);
        }
        Ok(res)
    }

    fn decode_bit<R: Read>(&mut self, reader: R, prob: &mut u16) -> std::io::Result<bool> {
        let bound = (self.range >> K_NUM_BIT_MODEL_TOTAL_BITS) * (*prob as u32);
        let symbol = if self.code < bound {
            *prob += ((1 << K_NUM_BIT_MODEL_TOTAL_BITS) - *prob) >> K_NUM_MOVE_BITS;
            self.range = bound;
            false
        } else {
            *prob -= *prob >> K_NUM_MOVE_BITS;
            self.code -= bound;
            self.range -= bound;
            true
        };
        self.normalize(reader)?;
        Ok(symbol)
    }
}

#[derive(Debug, Clone)]
struct BitTreeDecoder<const B: usize, const N: usize> {
    probs: [u16; N],
}

impl<const B: usize, const N: usize> Default for BitTreeDecoder<B, N> {
    fn default() -> Self {
        assert_eq!(1 << B, N);
        Self {
            probs: [PROB_INIT_VAL; N],
        }
    }
}

fn bit_tree_reverse_decode<R: Read>(
    mut reader: R,
    probs: &mut [u16],
    probs_offset: usize,
    num_bits: usize,
    range_decoder: &mut RangeDecoder,
) -> std::io::Result<u32> {
    let mut m: u32 = 1;
    let mut symbol: u32 = 0;
    for i in 0..num_bits {
        let bit = range_decoder.decode_bit(&mut reader, &mut probs[m as usize + probs_offset])?;
        m <<= 1;
        if bit {
            m |= 1;
            symbol |= 1 << i;
        }
    }
    Ok(symbol)
}

impl<const B: usize, const N: usize> BitTreeDecoder<B, N> {
    fn decode<R: Read>(&mut self, mut reader: R, rc: &mut RangeDecoder) -> std::io::Result<u32> {
        let mut m: u32 = 1;
        for _ in 0..B {
            if rc.decode_bit(&mut reader, &mut self.probs[m as usize])? {
                m <<= 1;
                m |= 1;
            } else {
                m <<= 1;
            }
        }
        Ok(m - (1 << B))
    }

    fn decode_reverse<R: Read>(
        &mut self,
        reader: R,
        rc: &mut RangeDecoder,
    ) -> std::io::Result<u32> {
        bit_tree_reverse_decode(reader, &mut self.probs, 0, B, rc)
    }
}

#[derive(Debug, Clone)]
struct LenDecoder {
    choice: u16,
    choice2: u16,
    low_coder: [BitTreeDecoder<3, 8>; 1 << K_NUM_POS_BITS_MAX],
    mid_coder: [BitTreeDecoder<3, 8>; 1 << K_NUM_POS_BITS_MAX],
    high_coder: BitTreeDecoder<8, 256>,
}

impl Default for LenDecoder {
    fn default() -> Self {
        Self {
            choice: PROB_INIT_VAL,
            choice2: PROB_INIT_VAL,
            low_coder: std::array::from_fn(|_| BitTreeDecoder::default()),
            mid_coder: std::array::from_fn(|_| BitTreeDecoder::default()),
            high_coder: BitTreeDecoder::default(),
        }
    }
}

impl LenDecoder {
    fn decode<R: Read>(
        &mut self,
        mut reader: R,
        rc: &mut RangeDecoder,
        pos_state: usize,
    ) -> std::io::Result<u32> {
        if !rc.decode_bit(&mut reader, &mut self.choice)? {
            return self.low_coder[pos_state].decode(&mut reader, rc);
        }
        if !rc.decode_bit(&mut reader, &mut self.choice2)? {
            return Ok(8 + self.mid_coder[pos_state].decode(&mut reader, rc)?);
        }
        Ok(16 + self.high_coder.decode(&mut reader, rc)?)
    }
}

fn update_state_literal(state: usize) -> usize {
    if state < 4 {
        0
    } else if state < 10 {
        state - 3
    } else {
        state - 6
    }
}
fn update_state_match(state: usize) -> usize {
    if state < 7 {
        7
    } else {
        10
    }
}
fn update_state_rep(state: usize) -> usize {
    if state < 7 {
        8
    } else {
        11
    }
}
fn update_state_short_rep(state: usize) -> usize {
    if state < 7 {
        9
    } else {
        11
    }
}

#[derive(Debug, Default, Clone)]
pub struct OutWindow {
    buf: Box<[u8]>,
    pos: usize,
    total_pos: usize,
    is_full: bool,
}

impl OutWindow {
    fn new(size: usize) -> Self {
        Self {
            buf: vec![0u8; size].into_boxed_slice(),
            pos: 0,
            total_pos: 0,
            is_full: false,
        }
    }

    fn put_byte<W: Write>(&mut self, mut writer: W, byte: u8) -> std::io::Result<()> {
        self.buf[self.pos] = byte;
        self.pos += 1;
        self.total_pos += 1;
        if self.pos == self.buf.len() {
            self.pos = 0;
            self.is_full = true;
        }
        writer.write_all(&[byte])?;
        Ok(())
    }

    fn get_byte(&self, dist: usize) -> std::io::Result<u8> {
        let index = if dist <= self.pos {
            self.pos - dist
        } else {
            self.buf.len() - dist + self.pos
        };
        self.buf.get(index).copied().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "LZMA OutWindow cannot get byte outside of window",
            )
        })
    }

    fn copy_match<W: Write>(
        &mut self,
        mut writer: W,
        dist: usize,
        length: usize,
    ) -> std::io::Result<()> {
        for _ in 0..length {
            let byte = self.get_byte(dist)?;
            self.put_byte(&mut writer, byte)?;
        }
        Ok(())
    }

    fn check_distance(&self, dist: usize) -> bool {
        dist <= self.pos || self.is_full
    }

    fn is_empty(&self) -> bool {
        self.pos == 0 && !self.is_full
    }
}

#[derive(Debug, Default, Clone)]
struct LzmaProps {
    lc: u8,
    pb: u8,
    lp: u8,
    #[allow(unused)]
    dict_size_in_props: u32,
    dict_size: u32,
}

impl LzmaProps {
    fn read<R: Read>(mut reader: R) -> std::io::Result<Self> {
        let d = u8::from_le_bytes(reader.read_const()?);
        if d >= (9 * 5 * 5) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Failed to read LZMA propterties",
            ));
        }

        let dict_size_in_props = u32::from_le_bytes(reader.read_const()?);

        Ok(Self {
            lc: d % 9,
            pb: (d / 9) / 5,
            lp: (d / 9) % 5,
            dict_size_in_props,
            dict_size: dict_size_in_props.max(LZMA_DICT_MIN),
        })
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub enum LzmaDecodeOptionSize {
    #[default]
    /// LZMA size is defined in the LZMA header, or end symbol inside the stream.
    ///
    /// If this is selected, header size (8 bytes) is always defined inside the header, but if it is
    /// u64::MAX, the decoder will instead use end symbol inside of the stream.
    Header,
    /// LZMA size is defined by end symbol inside the stream.
    None,
    /// LZMA size is custom defined.
    User(u64),
}

#[derive(Debug, Default, Clone, Copy)]
pub struct LzmaDecodeOptions {
    pub size: LzmaDecodeOptionSize,
    /// ZIP has an extended header format.
    /// See: https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT
    /// At section 5.8
    pub zip_format: bool,
}

#[derive(Debug, Clone)]
pub struct LzmaState {
    options: LzmaDecodeOptions,

    has_read_zip_information: bool,

    has_read_props: bool,
    props: LzmaProps,
    has_read_uncompressed_size: bool,
    uncompressed_size: Option<u64>,
    is_range_decoder_initialized: bool,

    unpack_size: Option<u64>,

    out_window: OutWindow,

    range_decoder: RangeDecoder,

    lit_probs: Box<[u16]>,

    pos_slot_decoder: [BitTreeDecoder<6, 64>; K_NUM_LEN_TO_POS_STATES as usize],
    align_decoder: BitTreeDecoder<K_NUM_ALIGN_BITS, K_NUM_ALIGN_BITS_COUNT>,
    pos_decoders: [u16; 1 + K_NUM_FULL_DISTANCES - K_END_POS_MODEL_INDEX],

    is_match: [u16; K_NUM_STATES << K_NUM_POS_BITS_MAX],
    is_rep: [u16; K_NUM_STATES],
    is_rep_g0: [u16; K_NUM_STATES],
    is_rep_g1: [u16; K_NUM_STATES],
    is_rep_g2: [u16; K_NUM_STATES],
    is_rep_0long: [u16; K_NUM_STATES << K_NUM_POS_BITS_MAX],

    len_decoder: LenDecoder,
    rep_len_decoder: LenDecoder,

    rep0: u32,
    rep1: u32,
    rep2: u32,
    rep3: u32,
    state: usize,

    out: VecDeque<u8>,
    finished: bool,
    corrupted: bool,
}

impl Default for LzmaState {
    fn default() -> Self {
        Self {
            options: LzmaDecodeOptions::default(),
            has_read_zip_information: false,
            has_read_props: false,
            props: LzmaProps::default(),
            has_read_uncompressed_size: false,
            uncompressed_size: None,
            is_range_decoder_initialized: false,
            unpack_size: None,
            out_window: OutWindow::default(),
            range_decoder: RangeDecoder::default(),
            lit_probs: Default::default(),
            pos_slot_decoder: Default::default(),
            align_decoder: Default::default(),
            pos_decoders: [PROB_INIT_VAL; 1 + K_NUM_FULL_DISTANCES - K_END_POS_MODEL_INDEX],
            is_match: [PROB_INIT_VAL; K_NUM_STATES << K_NUM_POS_BITS_MAX],
            is_rep: [PROB_INIT_VAL; K_NUM_STATES],
            is_rep_g0: [PROB_INIT_VAL; K_NUM_STATES],
            is_rep_g1: [PROB_INIT_VAL; K_NUM_STATES],
            is_rep_g2: [PROB_INIT_VAL; K_NUM_STATES],
            is_rep_0long: [PROB_INIT_VAL; K_NUM_STATES << K_NUM_POS_BITS_MAX],
            len_decoder: LenDecoder::default(),
            rep_len_decoder: LenDecoder::default(),
            rep0: 0,
            rep1: 0,
            rep2: 0,
            rep3: 0,
            state: 0,
            out: VecDeque::new(),
            finished: false,
            corrupted: false,
        }
    }
}

impl LzmaState {
    pub fn new(options: LzmaDecodeOptions) -> Self {
        Self {
            options,
            ..Self::default()
        }
    }

    /// Byte size of this struct in memory. (Probably not accurate, just close)
    /// Does not count bytes that have not yet been read from the internal out buffer.
    /// Will return None if hasn't read properties yet.
    pub fn decoder_byte_size(&self) -> Option<usize> {
        self.has_read_props
            .then_some(size_of::<Self>() + self.lit_probs.len() * 2 + self.out_window.buf.len())
    }

    fn read_zip_information<R: Read>(&mut self, mut reader: R) -> std::io::Result<()> {
        if !self.has_read_zip_information {
            self.has_read_zip_information = true;
            let _version = u16::from_le_bytes(reader.read_const()?);
            let header_size = u16::from_le_bytes(reader.read_const()?);
            if header_size != 5 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "LZMA zip header invalid header_size",
                ));
            }
        }
        Ok(())
    }

    fn read_props<R: Read>(&mut self, reader: R) -> std::io::Result<()> {
        if !self.has_read_props {
            self.has_read_props = true;
            self.props = LzmaProps::read(reader)?;
            self.out_window = OutWindow::new(self.props.dict_size as usize);
            self.lit_probs =
                vec![PROB_INIT_VAL; 0x300 << (self.props.lc + self.props.lp)].into_boxed_slice();
        }
        Ok(())
    }

    fn read_uncompressed_size<R: Read>(&mut self, mut reader: R) -> std::io::Result<()> {
        if !self.has_read_uncompressed_size {
            self.has_read_uncompressed_size = true;
            match self.options.size {
                LzmaDecodeOptionSize::Header => {
                    let uncompressed_size = u64::from_le_bytes(reader.read_const()?);
                    self.uncompressed_size =
                        (uncompressed_size != u64::MAX).then_some(uncompressed_size);
                }
                LzmaDecodeOptionSize::None => {
                    self.uncompressed_size = None;
                }
                LzmaDecodeOptionSize::User(size) => {
                    self.uncompressed_size = Some(size);
                }
            }
            self.unpack_size = self.uncompressed_size;
        }
        Ok(())
    }

    fn initialize_range_decoder<R: Read>(&mut self, mut reader: R) -> std::io::Result<()> {
        if !self.is_range_decoder_initialized {
            self.is_range_decoder_initialized = true;
            self.range_decoder.init(&mut reader)?;
        }
        Ok(())
    }

    fn initialize_if_needed<R: Read>(&mut self, mut reader: R) -> std::io::Result<()> {
        if self.options.zip_format && !self.has_read_zip_information {
            self.read_zip_information(&mut reader)?;
        }

        if !self.has_read_props {
            self.read_props(&mut reader)?;
        }

        if !self.has_read_uncompressed_size {
            self.read_uncompressed_size(&mut reader)?;
        }

        if !self.is_range_decoder_initialized {
            self.initialize_range_decoder(&mut reader)?;
        }

        Ok(())
    }

    fn decode_literal<R: Read>(&mut self, mut reader: R) -> std::io::Result<u8> {
        let mut prev_byte = 0;
        if !self.out_window.is_empty() {
            prev_byte = self.out_window.get_byte(1)?;
        }

        let mut symbol = 1;
        let lit_state = ((self.out_window.total_pos & ((1 << self.props.lp) - 1)) << self.props.lc)
            + ((prev_byte as usize) >> (8 - self.props.lc));
        let probs = &mut self.lit_probs[(0x300 * lit_state)..(0x300 * (lit_state + 1))];

        if self.state >= 7 {
            let mut match_byte = self.out_window.get_byte((self.rep0 as usize) + 1)? as usize;
            while symbol < 0x100 {
                let match_bit = (match_byte >> 7) & 1;
                match_byte <<= 1;
                let bit = self
                    .range_decoder
                    .decode_bit(&mut reader, &mut probs[((match_bit + 1) << 8) + symbol])?
                    as usize;
                symbol = (symbol << 1) | bit;
                if match_bit != bit {
                    break;
                }
            }
        }

        while symbol < 0x100 {
            let bit = self
                .range_decoder
                .decode_bit(&mut reader, &mut probs[symbol])? as usize;
            symbol = (symbol << 1) | bit;
        }

        Ok((symbol - 0x100) as u8)
    }

    fn decode_distance<R: Read>(&mut self, mut reader: R, len: u32) -> std::io::Result<u32> {
        let len_state = len.min(K_NUM_LEN_TO_POS_STATES - 1);

        let pos_slot = self.pos_slot_decoder[len_state as usize]
            .decode(&mut reader, &mut self.range_decoder)?;
        if pos_slot < 4 {
            return Ok(pos_slot);
        }

        let num_direct_bits = (pos_slot >> 1) - 1;
        let mut dist = (2 | (pos_slot & 1)) << num_direct_bits;
        if pos_slot < K_END_POS_MODEL_INDEX as u32 {
            dist += bit_tree_reverse_decode(
                &mut reader,
                &mut self.pos_decoders,
                (dist - pos_slot) as usize,
                num_direct_bits as usize,
                &mut self.range_decoder,
            )?;
        } else {
            dist += self
                .range_decoder
                .decode_direct_bits(&mut reader, num_direct_bits - (K_NUM_ALIGN_BITS as u32))?
                << K_NUM_ALIGN_BITS;
            dist += self
                .align_decoder
                .decode_reverse(&mut reader, &mut self.range_decoder)?;
        }

        Ok(dist)
    }

    fn update_unpack_size(&mut self, amount: u64) -> std::io::Result<()> {
        if let Some(unpack_size) = self.unpack_size.as_mut() {
            if amount > *unpack_size {
                return Err(std::io::Error::other("LZMA unpack size is invalid"));
            }
            *unpack_size -= amount;
            if *unpack_size == 0 {
                self.finished = true;
                if !self.range_decoder.is_finished() {
                    log::warn!(
                        "Finished unpacking LZMA, but range decoder has invalid finish state"
                    );
                }
            }
        }
        Ok(())
    }

    fn decode_next_packet<R: Read>(&mut self, mut reader: R) -> std::io::Result<()> {
        // Decode next packet:
        // https://en.wikipedia.org/wiki/LZMA#Compressed_format_overview

        if self.finished {
            return Ok(());
        }

        let pos_state = self.out_window.total_pos & ((1 << self.props.pb) - 1);

        if !self.range_decoder.decode_bit(
            &mut reader,
            &mut self.is_match[(self.state << K_NUM_POS_BITS_MAX) + pos_state],
        )? {
            // PACKET LIT

            let literal = self.decode_literal(&mut reader)?;
            self.out_window.put_byte(&mut self.out, literal)?;
            self.state = update_state_literal(self.state);
            self.update_unpack_size(1)?;

            return Ok(());
        }

        let mut len: u32;

        if self
            .range_decoder
            .decode_bit(&mut reader, &mut self.is_rep[self.state])?
        {
            // PACKET SHORTREP or LONGREP[*]

            if self.out_window.is_empty() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Invalid LZMA stream (LZMA.out_window was expected to not be empty)",
                ));
            }

            if !self
                .range_decoder
                .decode_bit(&mut reader, &mut self.is_rep_g0[self.state])?
            {
                if !self.range_decoder.decode_bit(
                    &mut reader,
                    &mut self.is_rep_0long[(self.state << K_NUM_POS_BITS_MAX) + pos_state],
                )? {
                    // PACKET SHORTREP

                    self.state = update_state_short_rep(self.state);
                    let byte = self.out_window.get_byte((self.rep0 as usize) + 1)?;
                    self.out_window.put_byte(&mut self.out, byte)?;
                    self.update_unpack_size(1)?;
                    return Ok(());
                } else {
                    // PACKET LONGREP[0]
                }
            } else {
                let dist: u32;
                if !self
                    .range_decoder
                    .decode_bit(&mut reader, &mut self.is_rep_g1[self.state])?
                {
                    // PACKET LONGREP[1]

                    dist = self.rep1;
                } else {
                    if !self
                        .range_decoder
                        .decode_bit(&mut reader, &mut self.is_rep_g2[self.state])?
                    {
                        // PACKET LONGREP[2]

                        dist = self.rep2;
                    } else {
                        // PACKET LONGREP[3]

                        dist = self.rep3;
                        self.rep3 = self.rep2;
                    }
                    self.rep2 = self.rep1;
                }
                self.rep1 = self.rep0;
                self.rep0 = dist;
            }
            len = self
                .rep_len_decoder
                .decode(&mut reader, &mut self.range_decoder, pos_state)?;
            self.state = update_state_rep(self.state);
        } else {
            // PACKET MATCH

            self.rep3 = self.rep2;
            self.rep2 = self.rep1;
            self.rep1 = self.rep0;
            len = self
                .len_decoder
                .decode(&mut reader, &mut self.range_decoder, pos_state)?;
            self.state = update_state_match(self.state);
            self.rep0 = self.decode_distance(&mut reader, len)?;
            if self.rep0 == 0xFFFFFFFF {
                if self.range_decoder.is_finished() {
                    self.finished = true;
                    return Ok(());
                } else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "LZMA invalid end of stream",
                    ));
                }
            }
            if self.rep0 >= self.props.dict_size
                || !self.out_window.check_distance(self.rep0 as usize)
            {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "LZMA tried to LZ data outside of dict",
                ));
            }
        }

        len += K_MATCH_MIN_LEN;

        self.out_window
            .copy_match(&mut self.out, (self.rep0 as usize) + 1, len as usize)?;
        self.update_unpack_size(len as u64)?;

        Ok(())
    }

    fn decode_next_inner<R: Read>(&mut self, mut reader: R) -> std::io::Result<()> {
        if self.finished {
            return Ok(());
        }

        self.initialize_if_needed(&mut reader)?;
        self.decode_next_packet(&mut reader)?;

        Ok(())
    }

    fn decode_next<R: Read>(&mut self, reader: R) -> std::io::Result<()> {
        if self.corrupted {
            return Err(std::io::Error::other(
                "LZMA stream is corrupted (See previous error)",
            ));
        }
        if let Err(err) = self.decode_next_inner(reader) {
            log::error!("LZMA ERROR: {err}");
            log::error!(
                "LZMA INFO:\nfinished: {}\nrange_decoder: {:?}",
                self.finished,
                self.range_decoder,
            );
            self.corrupted = true;
            return Err(err);
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct LzmaDecoder<'a, R: Read> {
    inner: R,
    state: &'a mut LzmaState,
}

impl<'a, R: Read> LzmaDecoder<'a, R> {
    pub fn new(inner: R, state: &'a mut LzmaState) -> Self {
        Self { inner, state }
    }
}

impl<'a, R: Read> Read for LzmaDecoder<'a, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let buf_len = buf.len();
        let mut write_len: usize = 0;

        while write_len < buf_len {
            if let Some(byte) = self.state.out.pop_front() {
                buf[write_len] = byte;
                write_len += 1;
            } else {
                if self.state.finished {
                    break;
                }
                self.state.decode_next(&mut self.inner)?;
            }
        }

        Ok(write_len)
    }
}

pub struct LzmaDecoderOwned<R: Read> {
    inner: R,
    state: LzmaState,
}

impl<R: Read> LzmaDecoderOwned<R> {
    pub fn new(inner: R, state: LzmaState) -> Self {
        Self { inner, state }
    }
}

impl<R: Read> Read for LzmaDecoderOwned<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        LzmaDecoder::new(&mut self.inner, &mut self.state).read(buf)
    }
}

#[cfg(test)]
mod test {
    use std::io::Read;

    use crate::LzmaState;

    use super::LzmaDecoderOwned;

    fn do_test(original: &[u8], compressed: &[u8]) -> std::io::Result<()> {
        let mut decoder =
            LzmaDecoderOwned::new(std::io::Cursor::new(&compressed), LzmaState::default());
        let mut decoded = vec![];
        decoder.read_to_end(&mut decoded)?;

        assert_eq!(&decoded, original);

        Ok(())
    }

    #[test]
    fn lzma_simple_test() -> std::io::Result<()> {
        // Literals only: https://github.com/google/wuffs/blob/main/lib/litonlylzma/example_test.go
        do_test(
            b"Hello world.\n",
            &[
                0x5d, 0x00, 0x10, 0x00, 0x00, 0x0d, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x24, 0x19, 0x49, 0x86, 0xe7, 0xd5, 0xe5, 0xe2, 0x56, 0xed, 0x6a, 0xe6, 0x0e, 0x81,
                0xfe, 0xb8, 0x00, 0x00,
            ],
        )?;
        // Should be only literals
        do_test(
            b"Nya~Meow :3",
            &[
                0x5d, 0x00, 0x00, 0x00, 0x02, 0x0b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x27, 0x1e, 0x48, 0x28, 0x20, 0xa8, 0xd2, 0xe6, 0x5b, 0xe3, 0xf1, 0x6f, 0x3b, 0x5d,
                0x4c, 0x7f, 0xff, 0xec, 0xcb, 0x80, 0x00,
            ],
        )?;
        do_test(
            b"Hello, World!",
            &[
                0x5d, 0x00, 0x00, 0x01, 0x00, 0x0d, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x24, 0x19, 0x49, 0x98, 0x6f, 0x16, 0x02, 0x89, 0x0a, 0x98, 0xe7, 0x3f, 0xa8, 0xc3,
                0x95, 0x48, 0x4d, 0xff, 0xff, 0x75, 0xf0, 0x00, 0x00,
            ],
        )?;
        Ok(())
    }
}
