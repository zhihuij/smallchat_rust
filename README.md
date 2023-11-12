# Smallchat with Rust

Inspired by [antirez/smallchat][smallchat].

## Usage

Run the server:
```shell
cargo run smallchat_server
```

Connect to the server with the client:
```shell
cargo run smallchat_server 127.0.0.1 7711
```

## History

* [v0.0.1][v0.0.1]: Single thread version with std::net;
* [v0.0.2][v0.0.2]: Single thread version with mio;
* [v0.0.3][v0.0.3]: Client with raw mode console.

## License

This project is licensed under the [MIT license][license].

[license]: https://github.com/zhihuij/smallchat_rust/blob/main/LICENSE
[smallchat]: https://github.com/antirez/smallchat
[v0.0.1]: https://github.com/zhihuij/smallchat_rust/tree/v0.0.1
[v0.0.2]: https://github.com/zhihuij/smallchat_rust/tree/v0.0.2
[v0.0.3]: https://github.com/zhihuij/smallchat_rust/tree/main
