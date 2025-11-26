# TCPTalk

## About

A neat little party trick that you can show to your developer friends is that you can chat with each other in the terminal using [netcat](https://en.wikipedia.org/wiki/Netcat).

All you have to do is setup the server (`nc -l 0.0.0.0 3000`) and connect to it via the client (`nc 0.0.0.0 3000`). See below.

![EI292CUR](https://github.com/user-attachments/assets/8340d93e-0c1b-4cff-9adf-742f26578f0f)

However, only 1:1 connections are supported, so chatting with more than 2 people is not possible.

This small little experiment decides to expand on the netcat party trick by providing a server that supports multiple connections at once and a fully fledged TUI client program that connects to said server.

https://github.com/user-attachments/assets/34b2399a-606d-4d51-9d3d-206ffa279c5a

In essence, a TCP chatroom for you and your friends.

## How to Setup

WIP.

## 👾 Bugs or vulnerabilities

If you find any bugs or vulnerabilities, please contact me on my Twitter using the link below.

_Made with ❤️ by [krayondev](https://x.com/krayondev)_
