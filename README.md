# TAIL TALES

A TUI log parser written in Rust

## Objetives

To easy inspect existing logs or streaming logs, with some filters, marking of messages
and easy finding information in the logs.

## Features

- [ ] TUI
- [ ] Read a log file and be able to navigate using arrows and simple search commands
- [ ] Filter messages with some simple expressions (regex? just text?)ç
- [ ] Parse the lines and provide a formated simple dictionary (key:value). Can see
      the log lines and the formatted data. Firswt version bassed on patterns.
- [ ] Filtering language similar to logql from loki.
- [ ] Logfmt format parsing
- [ ] Filtering and marking based on these filters
- [ ] Basic statistics and graphs
- [ ] Parsing of many lines in parallel
- [ ] Streaming input, should be use dexactly the same way. Can be from stdin, or appending file (tail style).
- [ ] Parsing of journald
- [ ] As it may have many many lines, be able to scroll efficiently
- [ ] Never blocking
