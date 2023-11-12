# Smallchat with Rust

Inspired by [antirez/smallchat][smallchat].

## Usage

Run the server:
```shell
cargo run --bin smallchat_server
```

Connect to the server with the client(can't be run inside IDE):
```shell
cargo run --bin smallchat_client 127.0.0.1 7711
```
then you can chat like in an IRC:
```shell
Welcome to Simple Chat! Use /nick <nick> to set your nick.
you> hello
user:5> hi
you> how are you?
user:5> fine, it's cold~
what' the whether like?
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
