<div align="center">
    <img src="./images/crab_coffee.png">
</div>

This is the rustenuine firmware for the [Chew](https://github.com/flinguenheld/chew) keyboard!

It performs some cool hacks that keyboard enthusiasts love:

- Layers
    - Set default
    - Activated as long as a key is held down (the best ❤️)
    - Dead key style (activated until another key is pressed)
- Homerows
    - Hold down to activate a modifier
    - Press to print a regular key
- Combos
    - Two keys pressed at once result in a third one
- Leader key
    - Once activated, it allows you to hit a sequence (3 keys max) to produce another key
    - Leave it with Escape or a wrong key
- Macros
    - One key can produce a chain of keys (e.g. to automate accents or email addresses)
- Mouse emulation
- No mods
    - Prevent modifiers with specific keys (e.g. no shifted symbols)
- Caplock
    - Deactivated by itself or Escape

<div align="center">
    <img src="./images/prawns.png">
</div>

### Layout

Here's my *current* layout which uses the US international extended keymap.  
The letter positions come from [Ergo-L](https://ergol.org/) which is the best French layout.

<div align="center">
    <img src="./images/layouts.png">
</div>

<div align="center">
    <img src="./images/combos.png">
</div>

<div align="center">
    <img src="./images/prawns.png">
</div>

### Install

Clone this repo:

```
  git clone https://github.com/flinguenheld/rustychew
```

Then, hold down the controller's **boot** button and plug in the usb cable.  
Mount the controller's drive and use one of these commands:

```
  cargo run --release --bin mono
```
```
  cargo run --release --bin split
```

> 💡 *The [0xCB-Gemini](https://github.com/0xCB-dev/0xCB-Gemini) controller has a vbus detection that allows the keyboard to know which side is
connected to the computer.  
> Therefore once you have flashed both sides with the split command, you'll be able to use any of them as master.*

<div align="center">
    <img src="./images/prawns.png">
</div>

### Thanks

Rusty Chew uses the [usbd-human-interface-device](https://github.com/dlkj/usbd-human-interface-device) crate which simplifies
the usb management **a lot** ❤️.  
Also the [pio-uart](https://github.com/Sympatron/pio-uart) which allowed me to create a UART half-duplex protocol.  
And the [rp-hal-boards](https://github.com/rp-rs/rp-hal-boards) that I used as a base to use the RP2040-zero controller.  

<div align="center">
    <img src="./images/crab_back.png">
</div>
