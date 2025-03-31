alias r := run

run *args:
    http_proxy="http://127.0.0.1:7890" https_proxy="http://127.0.0.1:7890" cargo run -- {{args}}
