# TAIL TALES

A TUI log parser written in Rust

## Objetives

To easy inspect existing logs or streaming logs, with some filters, marking of messages
and easy finding information in the logs.

## Use

- Can read exisitng files, checks for changes: `tt /var/log/messages`
- Can be used as pipe destination: `journalctl -f | tt`

## Keybindings

- Arrows - Move around
- PageUp | PageDown - Move Around
- `/` Search
- `f` Filter
- `q` Quit
- `F1` Open a web browser to search for the current selected line (slightly anonymzed).
- `numbers + G` Goto that line number

## Settings

It comes with some sensible default settings from the settings.yaml file. It can overwriten, by section at
`~/.config/tailtales/settings.yaml` or the appropiate XDG config directory. See the file for further information.

## Search / filter language

All record liens are logfmt parsed. More parsers may come in the future.

Its possible to search and filter based on both the lien and the structured parsed contents.

- Just `text` will look for text
- The proper way is `"text"`, but if just a simple text is given its understood as text (converts the variable name to a string)
- `~ regex` or better `~ "regex"` can also be used to search / filter by regex
- Basic operations as >, <, >=, <=, ==, != between variablers (record fields) and strings or numbers

## Example expressions

- `INFO`
- `"^INFO` -- The closing " is assumed
- `timestamp <= "2025-01-01"` -- The comparison is string based, so timestamps better in ISO format
- `line_number > 1000 && line_number < 2000` -- TODO, no priority, no parenteheiss, will not work

## Features

- [x] TUI
- [x] Read a log file and be able to navigate using arrows and simple search commands
- [x] Filter messages with some simple expressions (regex? just text?)
- [x] Parse the lines and provide a formated simple dictionary (key:value). Can see
      the log lines and the formatted data. Firswt version bassed on patterns.
- [x] Filtering language similar to logql from loki.
- [x] Logfmt format parsing
- [x] Filtering and marking based on these filters
- [ ] Basic statistics and graphs
- [x] Parsing of many lines in parallel
- [x] Streaming input, should be use dexactly the same way. Can be from stdin, or appending file (tail style).
- [ ] Parsing of journald
- [x] As it may have many many lines, be able to scroll efficiently
- [ ] Never blocking
- [ ] Editable line parsers
