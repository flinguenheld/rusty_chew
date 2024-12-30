use usbd_human_interface_device::page::Keyboard;

pub enum Key {
    None,
    Std(Keyboard),
    Shifted(Keyboard),
    NeverShifted(Keyboard),
    Layout(u8),
    HR((Keyboard, Keyboard)),
}

// pub to_keys(key: Key) {

// }
