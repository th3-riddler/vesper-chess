#[derive(Copy, Clone, Eq, Hash, PartialEq, Debug)]
pub struct Move(u16);

#[repr(u8)]
pub enum MoveFlag {
    Quiet               = 0b0000,
    DoublePush          = 0b0001,
    
    KingSideCastle      = 0b0010,
    QueenSideCastle     = 0b0011,

    Capture             = 0b0100,
    EnPassant           = 0b0101,

    PromotionN          = 0b1000,
    PromotionB          = 0b1001,
    PromotionR          = 0b1010,
    PromotionQ          = 0b1011,

    PromotionCaptureN   = 0b1100,
    PromotionCaptureB   = 0b1101,
    PromotionCaptureR   = 0b1110,
    PromotionCaptureQ   = 0b1111,
}

impl Move {
    pub const NULL: Self = Self(0);

    pub const fn new(from: u8, to: u8, flag: MoveFlag) -> Self {
        Self(from as u16 | (to as u16) << 6 | (flag as u16) << 12)
    }

    pub const fn get_from(self) -> u8 { (self.0 & 0x3F) as u8 }
    pub const fn get_to(self) -> u8 { ((self.0 >> 6) & 0x3F) as u8 }
    
    pub const fn get_flag(self) -> MoveFlag { 
        match (self.0 >> 12) & 0x0F {
            0b0000 => MoveFlag::Quiet,
            0b0001 => MoveFlag::DoublePush,
            0b0010 => MoveFlag::KingSideCastle,
            0b0011 => MoveFlag::QueenSideCastle,
            0b0100 => MoveFlag::Capture,
            0b0101 => MoveFlag::EnPassant,
            0b1000 => MoveFlag::PromotionN,
            0b1001 => MoveFlag::PromotionB,
            0b1010 => MoveFlag::PromotionR,
            0b1011 => MoveFlag::PromotionQ,
            0b1100 => MoveFlag::PromotionCaptureN,
            0b1101 => MoveFlag::PromotionCaptureB,
            0b1110 => MoveFlag::PromotionCaptureR,
            0b1111 => MoveFlag::PromotionCaptureQ,
            _      => unreachable!(),
        }
    }
}