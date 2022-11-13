# prettycbor

Pretty-print (diagnostic) CBOR, optionally using [`cbor2diag.rb`](https://github.com/cabo/cbor-diag).

Install `cbor2diag.rb` using `gem install cbor-diag`.

## Installation
```shell
git clone https://github.com/sietseringers/prettycbor
cargo install --path prettycbor
```

## Usage
```
Pretty-print (diagnostic) CBOR, optionally using cbor2diag.rb

Usage: prettycbor [OPTIONS] [DATA]

Arguments:
  [DATA]  Data to act on, either hexadecimal or diagnostic. If absent, stdin is read.
           If neither --hex or --diag is given, the input is parsed as hexadecimal.
           If that works, the result is passed through cbor2diag.rb and then acted upon.
           If not, the input is acting upon directly

Options:
  -e, --embedded         Let cbor2diag.rb parse embedded CBOR using its -e flag
  -i, --indent <INDENT>  Amount of spaces used for indentation [default: 2]
  -x, --hex              Force parsing input as hexadecimal which is passed through cbor2diag.rb
  -d, --diag             Force acting directly on the input
  -h, --help             Print help information
  -V, --version          Print version information
```