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

If you want to edit  assemblyscript files, just edit them in `assembly/assembly` folder.

Before buiild wasm, need run 

```
npm i
```

to install modules

Then run:

```
npm run asbuild
```

to build wasm.

After build, copy `assembly/assembly/release.wasm` to `assets` folder.