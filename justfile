nl := "\n->"

project_dir := justfile_directory()

_build-wasm:
    @echo -e "{{nl}} compiling wasm"
    wasm-pack build --target web --no-typescript --no-pack --no-opt --out-dir {{project_dir}}/dist/site site -- --features hot-reload
_build-wasm-release:
    @echo -e "{{nl}} compiling wasm"
    wasm-pack build --target web --no-typescript --no-pack --out-dir {{project_dir}}/dist/site site

[parallel]
build-site:    _build-wasm

build-server: 
    @echo -e "{{nl}} compiling server"
    cargo build --bin server
_build-server-release:
    @echo -e "{{nl}} compiling server"
    cargo build --bin server --release


build:         build-site build-server
build-release: _build-wasm-release _build-server-release

run: build
    @echo -e "{{nl}} running"
    cargo run --bin server
run-release: build-release
    @echo -e "{{nl}} running"
    cargo run --bin server --release

_watch_site:
    @watchexec --exts rs,toml,scss,png -r -w site -w common -w wasm_bridge -- just build-site 
_watch_server:
    @watchexec --exts rs,toml -r -w server -w common -- cargo run --bin server
    
[parallel]
watch: _watch_server _watch_site
