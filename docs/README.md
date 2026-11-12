<div align="center">
  <img src="../images/crab_coffee.png">
</div>

The docs of the Chew firmware, one file per topic. The words of the chip are
in [RP2040.md](./RP2040.md), the other docs link to it.

### The Chip

| Doc                              | What                                                       |
| -------------------------------- | ---------------------------------------------------------- |
| [RP2040.md](./RP2040.md)         | the chip, its parts, and the vocabulary of all the docs    |
| [CLOCKS.md](./CLOCKS.md)         | the crystal, the ROSC, the PLLs and the clock tree         |
| [INTERRUPTS.md](./INTERRUPTS.md) | the vector table, `bind_interrupts!`, the bound lines      |

### The Firmware

| Doc                            | What                                                         |
| ------------------------------ | ------------------------------------------------------------ |
| [EMBASSY.md](./EMBASSY.md)     | the crates, `init`, the executor, the tasks, sharing       |
| [LIFECYCLE.md](./LIFECYCLE.md) | from power on to the tasks, then the run loop                |
| [BUILD.md](./BUILD.md)         | `.cargo/config.toml`, `memory.x`, the profiles, flashing     |

<div align="center">
  <img src="../images/prawns.png">
</div>

### The Parts

| Doc                              | What                                                       |
| -------------------------------- | ---------------------------------------------------------- |
| [LED.md](./LED.md)               | the WS2812 on GP16: colors, fade, gamma                    |
| [USB.md](./USB.md)               | `embassy-usb`: the setup, the descriptors, the tasks       |
| [SERIAL.md](./SERIAL.md)         | the serial in the firmware: queues, lines, echo, commands  |
| [SERIAL_PC.md](./SERIAL_PC.md)   | reading the USB serial from the PC                         |

### The Protocols

| Doc                    | What                                                         |
| ---------------------- | ------------------------------------------------------------ |
| [UART.md](./UART.md)   | a real serial port: the frame, the receiving, the speed      |

### Progress

| Doc                  | What                                                           |
| -------------------- | -------------------------------------------------------------- |
| [TODO.md](./TODO.md) | the plan of the port and what is done                          |

<div align="center">
  <img src="../images/prawns.png">
</div>

### Where to Start

```text
 RP2040 ──> LIFECYCLE ──> EMBASSY ──> INTERRUPTS ──> a part (LED, USB…)
 the words   the big       the        how a task
             picture       runtime    gets woken
```

<div align="center">
  <img src="../images/crab_back.png">
</div>
