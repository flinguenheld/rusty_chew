use usbd_human_interface_device::page::Keyboard;

pub enum Key {
    None,
    Std(Keyboard),
    Layout(u8),
}
