# TAIL TALES

A TUI log viewer written in Rust

## Objetives

To easy inspect existing logs or streaming logs, with some filters, marking of messages
and easy finding information in the logs.

## Use

- Can set default command or file to open: `tt`
- Can read exisitng files, checks for changes: `tt /var/log/messages`
- Can be used as pipe destination: `journalctl -f | tt`
- Can execute commands and show stdout / stderr: `tt !journalctl -f` -- AS bash does not like use of `!` in commands there is an alternative format: `tt -- journalctl -f`. Another option is `tt \!journalctl -f`.

## Commands

All keybindings are transalted to internal commands. It's possible to execute commands directly entering
command mode (: by default).

| Command             | Description                                                                                                                |
| ------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| quit                | Exit the application                                                                                                       |
| clear               | Refresh the screen, usefull when its damaged because of non controlled ANSI characters                                     |
| open_help           | Opens the help URL with teh current record original data, slightly anonymized                                              |
| open_url <url>      | Open a specified URL. Used in default keybindings to open help                                                             |
| command             | Opens command mode with a new command (not enough with `mode command` as that woudl not clear the current command)         |
| search_next         | Search for the next occurrence                                                                                             |
| search_prev         | Search for the previous occurrence                                                                                         |
| vmove <ammount>     | Move vertically, positive down, negative up                                                                                |
| hmove <ammount>     | Move horizontally, positive right, negative left                                                                           |
| vgoto <lineno>      | Goto to specific line number                                                                                               |
| clear_records       | Clear all records                                                                                                          |
| warning <msg...>    | Display a warning message                                                                                                  |
| toggle_mark <color> | Toggle a mark on the current line. Its a data for the record with that color as mark, and will be used at render.          |
| move_to_next_mark   | Move to the next marked line                                                                                               |
| move_to_prev_mark   | Move to the previous marked line                                                                                           |
| settings            | Open the xdg program with the local settings file. If the file does not exist yet it is created with the default settings. |
| mode <mode>         | Switch between different modes: normal, command, search or filter                                                          |
| toggle_details      | Toggle the display of details                                                                                              |

## Keybindings

| Key            | Command                                                |
| -------------- | ------------------------------------------------------ |
| :              | command                                                |
| \|             | mode filter                                            |
| f              | mode filter                                            |
| /              | mode search                                            |
| n              | search_next                                            |
| shift-n        | search_prev                                            |
| control-del    | clear_records                                          |
| control-l      | refresh_screen                                         |
| F1             | open_url https://github.com/davidmoreno/tailtales/#use |
| F2             | open_help                                              |
| q              | quit                                                   |
| control-c      | quit                                                   |
| o              | help                                                   |
| up             | vmove -1                                               |
| k              | vmove -1                                               |
| page up        | vmove -10                                              |
| down           | vmove 1                                                |
| j              | vmove 1                                                |
| page down      | vmove 10                                               |
| left           | hmove -1                                               |
| control-left   | hmove -10                                              |
| right          | hmove 1                                                |
| control-right  | hmove 10                                               |
| home           | vgoto 0                                                |
| end            | vgoto 2000000000                                       |
| G              | goto_line                                              |
| space          | toggle_mark yellow                                     |
| 1              | toggle_mark red                                        |
| 2              | toggle_mark green                                      |
| 3              | toggle_mark blue                                       |
| 4              | toggle_mark magenta                                    |
| 5              | toggle_mark cyan                                       |
| tab            | move_to_next_mark                                      |
| shift-back tab | move_to_prev_mark                                      |
| esc            | mode normal                                            |
| v              | toggle_details                                         |

## Settings

It comes with some sensible default settings from the settings.yaml file. It can overwriten, by section at
`~/.config/tailtales/settings.yaml` or the appropiate XDG config directory. See the file for further information.

## Rules

The settings file have several default rules, and more may be added (send your pull request with new file formats!).

It has a basic pattern on the file name to discover which rules to use and acording to the matrched rule it can set:

- Data extractors: as logfmt, patterns and regex. These extracted data allows easy filtering and search.
- Columns: From the extracted data, it can show some data into the columns.For example to easily format timestamp or processing time.
- Filters: Acording filters from the filtering language, allows to color the lines, or add a gutter (symbol at the left of the table).

## Filter Language

Its possible to search and filter based on both the line and the structured parsed contents.

- Just `text` will look for text
- The proper way is `"text"`, but if just a simple text is given its understood as text (converts the variable name to a string)
- `~ regex` or better `~ "regex"` can also be used to search / filter by regex
- Basic operations as >, <, >=, <=, ==, != between variablers (record fields) and strings or numbers

More will be added.

### Example expressions

- `INFO`
- `"^INFO` -- The closing " is assumed
- `timestamp <= "2025-01-01"` -- The comparison is string based, so timestamps better in ISO format
- `line_number > 1000 && line_number < 2000` -- TODO, no priority, no parenteheiss, will not work

## Features

- [x] TUI
- [x] Read a log file and be able to navigate using arrows and simple search commands
- [x] Read stdout/stderr from executed commands
- [x] Filter messages with some simple expressions
- [x] Parse the lines and provide a formated simple dictionary (key:value). Can see
      the log lines and the formatted data. Firswt version bassed on patterns.
- [x] Logfmt format parsing
- [x] Pattern format parsing
- [x] Regex format parsing
- [x] Filtering language
- [x] Filtering and marking based on these filters
- [x] Streaming input. Changes in the file, or pipe in, or executed command are seen inmediately.
- [x] As it may have many many lines, be able to scroll efficiently
- [x] Never blocking
