use crate::usb::types::SERIAL;

/// Answer the commands typed in the serial terminal, upper or lower case.
/// An unknown one prints the usage.
#[embassy_executor::task]
pub async fn answerphone() {
    loop {
        let mut msg = SERIAL.read().await;
        msg.make_ascii_lowercase();

        match msg.as_slice() {
            b"hello" => SERIAL.print(b"Hello!\n"),
            b"prout" => SERIAL.print(b"De mammouth !\n"),
            b"test" => SERIAL.print(b"Oh you want to test?\n"),
            _ => usage(),
        };
    }
}

/// Be careful about the lenght
const USAGE: &[u8] = b"
~~ USAGE ~~~~~~~~~~~~~~~~~~~~~~

   hello -> say hello!
   prout -> erk
   test  -> test

~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
";

/// Print the list of commands.
fn usage() {
    SERIAL.print(USAGE);
}
