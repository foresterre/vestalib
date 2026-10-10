/// See <https://docs.vestaboard.com/docs/characterCodes>
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(into = "u8", try_from = "u8")]
#[repr(u8)]
pub enum Character {
    Blank = 0,
    A = 1,
    B = 2,
    C = 3,
    D = 4,
    E = 5,
    F = 6,
    G = 7,
    H = 8,
    I = 9,
    J = 10,
    K = 11,
    L = 12,
    M = 13,
    N = 14,
    O = 15,
    P = 16,
    Q = 17,
    R = 18,
    S = 19,
    T = 20,
    U = 21,
    V = 22,
    W = 23,
    X = 24,
    Y = 25,
    Z = 26,
    One = 27,
    Two = 28,
    Three = 29,
    Four = 30,
    Five = 31,
    Six = 32,
    Seven = 33,
    Eight = 34,
    Nine = 35,
    Zero = 36,
    ExclamationMark = 37,
    At = 38,
    Pound = 39,
    Dollar = 40,
    LeftParenthesis = 41,
    RightParenthesis = 42,
    Hyphen = 44,
    Plus = 46,
    Ampersand = 47,
    Equal = 48,
    Semicolon = 49,
    Colon = 50,
    SingleQuote = 52,
    DoubleQuote = 53,
    Percent = 54,
    Comma = 55,
    Period = 56,
    Slash = 59,
    QuestionMark = 60,
    Degree = 62,
    Red = 63,
    Orange = 64,
    Yellow = 65,
    Green = 66,
    Blue = 67,
    Violet = 68,
    White = 69,
    Black = 70,
    Filled = 71,
}

impl Character {
    /// The Vestaboard Note shows a heart, where the Flagship shows a [`Character::Degree`].
    pub const HEART: Character = Character::Degree;

    pub fn code(self) -> u8 {
        self as u8
    }
}

impl From<Character> for u8 {
    fn from(character: Character) -> Self {
        character.code()
    }
}

impl TryFrom<u8> for Character {
    type Error = UnknownCharacter;

    fn try_from(code: u8) -> Result<Self, Self::Error> {
        use Character::*;

        let character = match code {
            0 => Blank,
            1 => A,
            2 => B,
            3 => C,
            4 => D,
            5 => E,
            6 => F,
            7 => G,
            8 => H,
            9 => I,
            10 => J,
            11 => K,
            12 => L,
            13 => M,
            14 => N,
            15 => O,
            16 => P,
            17 => Q,
            18 => R,
            19 => S,
            20 => T,
            21 => U,
            22 => V,
            23 => W,
            24 => X,
            25 => Y,
            26 => Z,
            27 => One,
            28 => Two,
            29 => Three,
            30 => Four,
            31 => Five,
            32 => Six,
            33 => Seven,
            34 => Eight,
            35 => Nine,
            36 => Zero,
            37 => ExclamationMark,
            38 => At,
            39 => Pound,
            40 => Dollar,
            41 => LeftParenthesis,
            42 => RightParenthesis,
            44 => Hyphen,
            46 => Plus,
            47 => Ampersand,
            48 => Equal,
            49 => Semicolon,
            50 => Colon,
            52 => SingleQuote,
            53 => DoubleQuote,
            54 => Percent,
            55 => Comma,
            56 => Period,
            59 => Slash,
            60 => QuestionMark,
            62 => Degree,
            63 => Red,
            64 => Orange,
            65 => Yellow,
            66 => Green,
            67 => Blue,
            68 => Violet,
            69 => White,
            70 => Black,
            71 => Filled,
            _ => return Err(UnknownCharacter { code }),
        };

        Ok(character)
    }
}

impl TryFrom<char> for Character {
    type Error = UnsupportedChar;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        use Character::*;

        let character = match value.to_ascii_uppercase() {
            ' ' => Blank,
            'A' => A,
            'B' => B,
            'C' => C,
            'D' => D,
            'E' => E,
            'F' => F,
            'G' => G,
            'H' => H,
            'I' => I,
            'J' => J,
            'K' => K,
            'L' => L,
            'M' => M,
            'N' => N,
            'O' => O,
            'P' => P,
            'Q' => Q,
            'R' => R,
            'S' => S,
            'T' => T,
            'U' => U,
            'V' => V,
            'W' => W,
            'X' => X,
            'Y' => Y,
            'Z' => Z,
            '1' => One,
            '2' => Two,
            '3' => Three,
            '4' => Four,
            '5' => Five,
            '6' => Six,
            '7' => Seven,
            '8' => Eight,
            '9' => Nine,
            '0' => Zero,
            '!' => ExclamationMark,
            '@' => At,
            '#' => Pound,
            '$' => Dollar,
            '(' => LeftParenthesis,
            ')' => RightParenthesis,
            '-' => Hyphen,
            '+' => Plus,
            '&' => Ampersand,
            '=' => Equal,
            ';' => Semicolon,
            ':' => Colon,
            '\'' => SingleQuote,
            '"' => DoubleQuote,
            '%' => Percent,
            ',' => Comma,
            '.' => Period,
            '/' => Slash,
            '?' => QuestionMark,
            '°' => Degree,
            _ => return Err(UnsupportedChar { value }),
        };

        Ok(character)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("Unknown character code '{}'", code)]
pub struct UnknownCharacter {
    pub code: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("The character '{}' can't be shown on a Vestaboard", value)]
pub struct UnsupportedChar {
    pub value: char,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_round_trip() {
        let known = (0..=u8::MAX)
            .filter_map(|code| Character::try_from(code).ok())
            .inspect(|character| assert_eq!(Character::try_from(character.code()), Ok(*character)))
            .count();

        assert_eq!(known, 66);
    }

    #[yare::parameterized(
        unused_43 = { 43 },
        unused_45 = { 45 },
        unused_51 = { 51 },
        unused_57 = { 57 },
        unused_58 = { 58 },
        unused_61 = { 61 },
        beyond_filled = { 72 },
        max = { 255 },
    )]
    fn unknown_codes(code: u8) {
        assert_eq!(Character::try_from(code), Err(UnknownCharacter { code }));
    }

    #[yare::parameterized(
        blank = { ' ', Character::Blank },
        upper = { 'H', Character::H },
        lower = { 'h', Character::H },
        one = { '1', Character::One },
        zero = { '0', Character::Zero },
        question_mark = { '?', Character::QuestionMark },
        single_quote = { '\'', Character::SingleQuote },
        degree = { '°', Character::Degree },
    )]
    fn from_char(value: char, expected: Character) {
        assert_eq!(Character::try_from(value), Ok(expected));
    }

    #[yare::parameterized(
        tab = { '\t' },
        newline = { '\n' },
        underscore = { '_' },
        accented = { 'é' },
        emoji = { '🦀' },
    )]
    fn unsupported_chars(value: char) {
        assert_eq!(Character::try_from(value), Err(UnsupportedChar { value }));
    }

    #[test]
    fn serde_as_code() {
        assert_eq!(serde_json::to_string(&Character::Red).unwrap(), "63");
        assert_eq!(
            serde_json::from_str::<Character>("62").unwrap(),
            Character::HEART
        );
        assert!(serde_json::from_str::<Character>("43").is_err());
        assert!(serde_json::from_str::<Character>("256").is_err());
    }
}
