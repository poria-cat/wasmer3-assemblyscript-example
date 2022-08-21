# Wasmer 3.0 and AssemblyScript example

Just run:

```
cargo run
```

Then will show:

```
Hello wasmer!
Hello AssemblyScript!
```

If you want to edit  assemblyscript files, just edit them in `assembly/assembly` folder

and run 

```
npm run asbuild
```

to build wasm

then copy `assembly/assembly/release.wasm`to `assets` folder