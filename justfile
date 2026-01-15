nl := "\n->"

project_dir := justfile_directory()

_build-css:
    @echo -e "{{nl}} compiling css"
    sass --embed-sources --load-path common/style common/style:{{project_dir}}/dist/style

_build-wasm:
    @echo -e "{{nl}} compiling wasm"
    wasm-pack build --target web --out-dir {{project_dir}}/dist/client client
_build-wasm-release:
    @echo -e "{{nl}} compiling wasm"
    wasm-pack build --target web --release --out-dir {{project_dir}}/dist/client client

_build-server:
    @echo -e "{{nl}} compiling server"
    cargo build --bin server
_build-server-release:
    @echo -e "{{nl}} compiling server"
    cargo build --bin server --release

build:         _build-css _build-wasm         _build-server
build-release: _build-css _build-wasm-release _build-server-release

run: build
    @echo -e "{{nl}} running"
    cargo run --bin server
run-release: build-release
    @echo -e "{{nl}} running"
    cargo run --bin server --release
    
watch:
    @watchexec --exts rs,toml,scss,png -r -- just run 
