<div align="center">
    <img src="./docs/images/crab_coffee.png">
</div>

This is the rustenuine firmware for the [Chew](https://github.com/flinguenheld/chew) keyboard!

<div align="center">
    <img src="./docs/images/prawns.png">
</div>

### Flash

Clone this repo:

```
  git clone https://github.com/flinguenheld/rusty_chew
```

Then, hold down the controller's **boot** button and plug in the usb cable.  
Mount the controller's drive and use one of these commands:  

> 💡 Use the feature "zero" to flash the RP2040-zero (its led is wired GRB).

##### Mono
```
  cargo run --release --bin mono
  cargo run --release --bin mono --features zero
```

##### Split with the [0xCB-Gemini](https://github.com/0xCB-dev/0xCB-Gemini) controller

>This controller has a vbus detection that allows the keyboard to know which side is
>connected to the computer.  
>Therefore once you have flashed both sides with the split command, you'll be able to use any of them as master.
```
  cargo run --release --bin split
```

##### Split with the [RP2040-Zéro](https://www.waveshare.com/wiki/RP2040-Zero) controller

>For this one, the position of the master is fixed, you have to make your choice and flash both side with one of these commands:

```
  cargo run --release --bin split --features master,zero
```
```
  cargo run --release --bin split --features slave,zero
```

##### Serial

The firmware prints text and answers commands on its USB serial (see [SERIAL.md](docs/SERIAL.md) and [SERIAL_PC.md](docs/SERIAL_PC.md)).  
Then check the usb events to know which tty is used.  
```
    sudo udevadm monitor
```

And connect to it (for instance with minicom or picocom. *Ctrl-A X to exit* )
```
    sudo minicom --device /dev/ttyACM0
    sudo picocom /dev/ttyACM0 -b 115200
```

<div align="center">
    <img src="./docs/images/prawns.png">
</div>

### Docs

The docs are Markdown files in [docs/](./docs/README.md), one per topic, readable
on GitHub as they are. [mdBook](https://rust-lang.github.io/mdBook/) turns them
into a book with a sidebar and a search.

##### Install

```
  cargo install mdbook
  cargo install md-tui --locked
```

##### Read

```
  mdbook serve --open      # in the browser, reloads on each save
  mdbook build             # or only build it, in target/book/
  mdt docs/SUMMARY.md      # in the terminal, from the summary
```

> 💡 In mdt: `s` selects a link, `Enter` opens it, `b` goes back, `q` quits.
> It needs a [Nerd Font](https://www.nerdfonts.com/) in the terminal.

##### Update

- Edit the files in `docs/`, images go in `docs/images/`.
- A new doc must be listed in [SUMMARY.md](./docs/SUMMARY.md) (the sidebar of
  the book) and in [docs/README.md](./docs/README.md) (the index on GitHub).

<div align="center">
    <img src="./docs/images/crab_back.png">
</div>
