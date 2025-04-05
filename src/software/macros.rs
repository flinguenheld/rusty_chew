use super::{
    keys::{Buffer, KC},
    modifiers::Modifiers,
};

/// Fill the buffer with all letters.
/// Check the letter list below.
pub fn str_to_usb_code(text: &str, mut buffer: Buffer) -> Buffer {
    let mut last = ' ';
    for c in text.chars() {
        if c == last {
            buffer = KC::None.usb_code(buffer, &Modifiers::new())
        }
        last = c;

        buffer = char_to_usb_code(c, buffer);
    }

    KC::None.usb_code(buffer, &Modifiers::new())
}

/// Add in the buffer the KC value of "symbol".
/// Use ↵ to enter.
fn char_to_usb_code(symbol: char, buffer: Buffer) -> Buffer {
    let no_mod = Modifiers::new();
    let mut shifted = Modifiers::new();
    shifted.set(KC::Sft, 0);

    match symbol {
        'a' => KC::A.usb_code(buffer, &no_mod),
        'b' => KC::B.usb_code(buffer, &no_mod),
        'c' => KC::C.usb_code(buffer, &no_mod),
        'd' => KC::D.usb_code(buffer, &no_mod),
        'e' => KC::E.usb_code(buffer, &no_mod),
        'f' => KC::F.usb_code(buffer, &no_mod),
        'g' => KC::G.usb_code(buffer, &no_mod),
        'h' => KC::H.usb_code(buffer, &no_mod),
        'i' => KC::I.usb_code(buffer, &no_mod),
        'j' => KC::J.usb_code(buffer, &no_mod),
        'k' => KC::K.usb_code(buffer, &no_mod),
        'l' => KC::L.usb_code(buffer, &no_mod),
        'm' => KC::M.usb_code(buffer, &no_mod),
        'n' => KC::N.usb_code(buffer, &no_mod),
        'o' => KC::O.usb_code(buffer, &no_mod),
        'p' => KC::P.usb_code(buffer, &no_mod),
        'q' => KC::Q.usb_code(buffer, &no_mod),
        'r' => KC::R.usb_code(buffer, &no_mod),
        's' => KC::S.usb_code(buffer, &no_mod),
        't' => KC::T.usb_code(buffer, &no_mod),
        'u' => KC::U.usb_code(buffer, &no_mod),
        'v' => KC::V.usb_code(buffer, &no_mod),
        'w' => KC::W.usb_code(buffer, &no_mod),
        'x' => KC::X.usb_code(buffer, &no_mod),
        'y' => KC::Y.usb_code(buffer, &no_mod),
        'z' => KC::Z.usb_code(buffer, &no_mod),

        'A' => KC::A.usb_code(buffer, &shifted),
        'B' => KC::B.usb_code(buffer, &shifted),
        'C' => KC::C.usb_code(buffer, &shifted),
        'D' => KC::D.usb_code(buffer, &shifted),
        'E' => KC::E.usb_code(buffer, &shifted),
        'F' => KC::F.usb_code(buffer, &shifted),
        'G' => KC::G.usb_code(buffer, &shifted),
        'H' => KC::H.usb_code(buffer, &shifted),
        'I' => KC::I.usb_code(buffer, &shifted),
        'J' => KC::J.usb_code(buffer, &shifted),
        'K' => KC::K.usb_code(buffer, &shifted),
        'L' => KC::L.usb_code(buffer, &shifted),
        'M' => KC::M.usb_code(buffer, &shifted),
        'N' => KC::N.usb_code(buffer, &shifted),
        'O' => KC::O.usb_code(buffer, &shifted),
        'P' => KC::P.usb_code(buffer, &shifted),
        'Q' => KC::Q.usb_code(buffer, &shifted),
        'R' => KC::R.usb_code(buffer, &shifted),
        'S' => KC::S.usb_code(buffer, &shifted),
        'T' => KC::T.usb_code(buffer, &shifted),
        'U' => KC::U.usb_code(buffer, &shifted),
        'V' => KC::V.usb_code(buffer, &shifted),
        'W' => KC::W.usb_code(buffer, &shifted),
        'X' => KC::X.usb_code(buffer, &shifted),
        'Y' => KC::Y.usb_code(buffer, &shifted),
        'Z' => KC::Z.usb_code(buffer, &shifted),

        '0' => KC::Num0.usb_code(buffer, &no_mod),
        '1' => KC::Num1.usb_code(buffer, &no_mod),
        '2' => KC::Num2.usb_code(buffer, &no_mod),
        '3' => KC::Num3.usb_code(buffer, &no_mod),
        '4' => KC::Num4.usb_code(buffer, &no_mod),
        '5' => KC::Num5.usb_code(buffer, &no_mod),
        '6' => KC::Num6.usb_code(buffer, &no_mod),
        '7' => KC::Num7.usb_code(buffer, &no_mod),
        '8' => KC::Num8.usb_code(buffer, &no_mod),
        '9' => KC::Num9.usb_code(buffer, &no_mod),

        '-' => KC::Minus.usb_code(buffer, &no_mod),
        '=' => KC::Equal.usb_code(buffer, &no_mod),
        '{' => KC::LeftBracket.usb_code(buffer, &no_mod),
        '}' => KC::RightBracket.usb_code(buffer, &no_mod),
        '\\' => KC::Backslash.usb_code(buffer, &no_mod),
        ';' => KC::SemiColon.usb_code(buffer, &no_mod),
        '\'' => KC::Quote.usb_code(buffer, &no_mod),
        '`' => KC::Grave.usb_code(buffer, &no_mod),
        ',' => KC::Comma.usb_code(buffer, &no_mod),
        '.' => KC::Dot.usb_code(buffer, &no_mod),
        '/' => KC::Slash.usb_code(buffer, &no_mod),

        '~' => KC::Tilde.usb_code(buffer, &no_mod),
        '!' => KC::Exclaim.usb_code(buffer, &no_mod),
        '@' => KC::At.usb_code(buffer, &no_mod),
        '#' => KC::Hash.usb_code(buffer, &no_mod),
        '$' => KC::Dollar.usb_code(buffer, &no_mod),
        '%' => KC::Percentage.usb_code(buffer, &no_mod),
        '^' => KC::Circumflex.usb_code(buffer, &no_mod),
        '&' => KC::Ampersand.usb_code(buffer, &no_mod),
        '*' => KC::Asterix.usb_code(buffer, &no_mod),
        '(' => KC::LeftParent.usb_code(buffer, &no_mod),
        ')' => KC::RightParent.usb_code(buffer, &no_mod),
        '_' => KC::Underscore.usb_code(buffer, &no_mod),
        '+' => KC::Plus.usb_code(buffer, &no_mod),
        '[' => KC::LeftBracket.usb_code(buffer, &no_mod),
        ']' => KC::RightBracket.usb_code(buffer, &no_mod),
        '|' => KC::Pipe.usb_code(buffer, &no_mod),
        ':' => KC::Colon.usb_code(buffer, &no_mod),
        '"' => KC::DoubleQuote.usb_code(buffer, &no_mod),
        '<' => KC::LowerThan.usb_code(buffer, &no_mod),
        '>' => KC::GreaterThan.usb_code(buffer, &no_mod),
        '?' => KC::Question.usb_code(buffer, &no_mod),

        '«' => KC::GuillemetL.usb_code(buffer, &no_mod),
        '»' => KC::GuillemetD.usb_code(buffer, &no_mod),
        'ø' => KC::Diameter.usb_code(buffer, &no_mod),
        '°' => KC::Degre.usb_code(buffer, &no_mod),
        '€' => KC::Euro.usb_code(buffer, &no_mod),
        '£' => KC::Pound.usb_code(buffer, &no_mod),

        '©' => KC::Copyright.usb_code(buffer, &no_mod),
        '®' => KC::Registered.usb_code(buffer, &no_mod),

        '¹' => KC::ExpOne.usb_code(buffer, &no_mod),
        '²' => KC::ExpTwo.usb_code(buffer, &no_mod),
        '³' => KC::ExpThree.usb_code(buffer, &no_mod),
        '¶' => KC::Pilcrow.usb_code(buffer, &no_mod),
        '×' => KC::Multi.usb_code(buffer, &no_mod),
        '÷' => KC::Div.usb_code(buffer, &no_mod),
        'ß' => KC::Beta.usb_code(buffer, &no_mod),

        '¼' => KC::Quarter.usb_code(buffer, &no_mod),
        '½' => KC::Half.usb_code(buffer, &no_mod),
        '¾' => KC::ThreeQuarter.usb_code(buffer, &no_mod),

        '¥' => KC::Yen.usb_code(buffer, &no_mod),

        // French --
        'ç' => KC::CCedilla.usb_code(buffer, &no_mod),
        'é' => KC::EAcute.usb_code(buffer, &no_mod),
        'œ' => KC::CCedilla.usb_code(buffer, &no_mod),
        'æ' => KC::CCedilla.usb_code(buffer, &no_mod),

        'â' => KC::ACircum.usb_code(buffer, &no_mod),
        'à' => KC::AGrave.usb_code(buffer, &no_mod),
        'ä' => KC::ADiaer.usb_code(buffer, &no_mod),
        'ê' => KC::ECircum.usb_code(buffer, &no_mod),
        'è' => KC::EGrave.usb_code(buffer, &no_mod),
        'ë' => KC::EDiaer.usb_code(buffer, &no_mod),
        'î' => KC::ICircum.usb_code(buffer, &no_mod),
        'ì' => KC::IGrave.usb_code(buffer, &no_mod),
        'ï' => KC::IDiaer.usb_code(buffer, &no_mod),
        'ô' => KC::OCircum.usb_code(buffer, &no_mod),
        'ò' => KC::OGrave.usb_code(buffer, &no_mod),
        'ö' => KC::ODiaer.usb_code(buffer, &no_mod),
        'û' => KC::UCircum.usb_code(buffer, &no_mod),
        'ù' => KC::UGrave.usb_code(buffer, &no_mod),
        'ü' => KC::UDiaer.usb_code(buffer, &no_mod),
        'ŷ' => KC::YCircum.usb_code(buffer, &no_mod),
        'ỳ' => KC::YGrave.usb_code(buffer, &no_mod),
        'ÿ' => KC::YDiaer.usb_code(buffer, &no_mod),

        'Ç' => KC::CCedilla.usb_code(buffer, &shifted),
        'É' => KC::EAcute.usb_code(buffer, &shifted),
        'Œ' => KC::CCedilla.usb_code(buffer, &shifted),
        'Æ' => KC::CCedilla.usb_code(buffer, &shifted),

        'Â' => KC::ACircum.usb_code(buffer, &shifted),
        'À' => KC::AGrave.usb_code(buffer, &shifted),
        'Ä' => KC::ADiaer.usb_code(buffer, &shifted),
        'Ê' => KC::ECircum.usb_code(buffer, &shifted),
        'È' => KC::EGrave.usb_code(buffer, &shifted),
        'Ë' => KC::EDiaer.usb_code(buffer, &shifted),
        'Î' => KC::ICircum.usb_code(buffer, &shifted),
        'Ì' => KC::IGrave.usb_code(buffer, &shifted),
        'Ï' => KC::IDiaer.usb_code(buffer, &shifted),
        'Ô' => KC::OCircum.usb_code(buffer, &shifted),
        'Ò' => KC::OGrave.usb_code(buffer, &shifted),
        'Ö' => KC::ODiaer.usb_code(buffer, &shifted),
        'Û' => KC::UCircum.usb_code(buffer, &shifted),
        'Ù' => KC::UGrave.usb_code(buffer, &shifted),
        'Ü' => KC::UDiaer.usb_code(buffer, &shifted),
        'Ŷ' => KC::YCircum.usb_code(buffer, &shifted),
        'Ỳ' => KC::YGrave.usb_code(buffer, &shifted),
        'Ÿ' => KC::YDiaer.usb_code(buffer, &shifted),

        '↵' => KC::Enter.usb_code(buffer, &shifted),
        _ => KC::Space.usb_code(buffer, &no_mod),
    }
}
