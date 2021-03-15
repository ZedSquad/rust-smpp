#[repr(u8)]
pub enum DeliverEsmClass {
    Default = (DeliverMessageMode::NotApplicable as u8
        | DeliverMessageType::Default as u8),
    SmscDeliveryReceipt = (DeliverMessageMode::NotApplicable as u8
        | DeliverMessageType::SmscDeliveryReceipt as u8),
}

#[repr(u8)]
enum DeliverMessageMode {
    // Significant bits: ........ (none)
    NotApplicable = 0b00000000,
}

/// https://smpp.org/SMPP_v3_4_Issue1_2.pdf section 5.2.12
#[allow(dead_code)]
#[repr(u8)]
enum DeliverMessageType {
    // Significant bits: ..0000.. (the middle 4)
    Default = 0b00000000,
    SmscDeliveryReceipt = 0b00000100,
    SmeDeliveryAcknowledgement = 0b00001000,
    SmeManualUserAcknowledgement = 0b00010000,
    ConversationAbort = 0b00011000,
    IntermediateDeliveryNotification = 0b00100000,
}

// TODO: GSM Network Specific Features (bits 7-6)
