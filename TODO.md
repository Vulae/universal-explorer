
# Features

- [ ] An interface that should have relevant extraction options for the files inside of that directory.
- [ ] CLI program
- [ ] Better icon
- [ ] Refactor explorer loading to allow for errorless handling on non-valid explorers
- [ ] Logs tab
- [ ] Move stuff to sub-modules to allow features & faster compiling for development.
- [ ] Multithreaded virtual_fs icons loading

# Formats

- [x] Non-animated images
- [ ] Animated images
- [ ] `.svg` vector image
- [x] Basic text files
    - [ ] Autodetect language for syntax highlighting
- [ ] Audio files
- [ ] Video files (Probably by piping to ffmplay)
- [ ] `.zip` archive
- [ ] Source engine
    - [x] `.vpk` archive
    - [x] `.vtf` texture
    - [ ] `.bsp` embedded `.zip`
- [ ] Godot engine
    - [x] `.pak` archive
        - [ ] `.exe` embedded `.pak` archive
    - [x] `.stex` stream texture [^godot-texture-partial-support]
    - [x] `.ctex` compressed texture [^godot-texture-partial-support]
    - [ ] Resource container
- [ ] Ren'Py engine
    - [x] `.rpa` archive
    - [ ] `.rpyc` script file decompilation
- [ ] Unity engine
- [ ] Unreal engine
    * (https://github.com/trumank/repak/tree/master)
    * (https://github.com/bananaturtlesandwich/unpak/tree/master)
- [ ] GameMaker engine

[^godot-texture-partial-support]: Partial support. Some format edge cases & no mipmap support.
