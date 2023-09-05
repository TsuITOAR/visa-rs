#![allow(overflowing_literals)]
#![allow(non_upper_case_globals)]

// pub const VI_GPIB_ATN_DEASSERT: ViUInt32 = 0;
// pub const VI_GPIB_ATN_ASSERT: ViUInt32 = 1;
// pub const VI_GPIB_ATN_DEASSERT_HANDSHAKE: ViUInt32 = 2;
// pub const VI_GPIB_ATN_ASSERT_IMMEDIATE: ViUInt32 = 3;

consts_to_enum! {
    #[format=dbg]
    #[repr(ViUInt16)]
    /// Specify whether the local interface should acquire or release Controller Active status
    ///
    /// See [`gpib_control_atn`](crate::Instrument::gpib_control_atn)
    ///
    pub enum AtnMode {
        VI_GPIB_ATN_DEASSERT            0   "Deassert ATN line. The GPIB interface corresponding to the VISA session goes to standby."
        VI_GPIB_ATN_ASSERT              1   "Assert ATN line and take control synchronously without corrupting transferred data. If a data handshake is in progress, ATN is not asserted until the handshake is complete."
        VI_GPIB_ATN_DEASSERT_HANDSHAKE  2   "Assert ATN line and take control synchronously without corrupting transferred data. If a data handshake is in progress, ATN is not asserted until the handshake is complete."
        VI_GPIB_ATN_ASSERT_IMMEDIATE    3   "Assert ATN line and take control asynchronously and immediately without regard for any data transfer currently in progress. Generally, this should be used only under error conditions."
    }
}

// pub const VI_GPIB_REN_DEASSERT: ViUInt32 = 0;
// pub const VI_GPIB_REN_ASSERT: ViUInt32 = 1;
// pub const VI_GPIB_REN_DEASSERT_GTL: ViUInt32 = 2;
// pub const VI_GPIB_REN_ASSERT_ADDRESS: ViUInt32 = 3;
// pub const VI_GPIB_REN_ASSERT_LLO: ViUInt32 = 4;
// pub const VI_GPIB_REN_ASSERT_ADDRESS_LLO: ViUInt32 = 5;
// pub const VI_GPIB_REN_ADDRESS_GTL: ViUInt32 = 6;

consts_to_enum! {
    #[format=dbg]
    #[repr(ViUInt16)]
    /// Asserts or unasserts the GPIB REN interface line
    ///
    /// See [`gpib_control_ren`](crate::Instrument::gpib_control_ren)
    ///
    pub enum RenMode {
        VI_GPIB_REN_DEASSERT            0   "Deassert REN line."
        VI_GPIB_REN_ASSERT              1   "Assert REN line."
        VI_GPIB_REN_DEASSERT_GTL        2   "Send the Go To Local (GTL) command and deassert REN line."
        VI_GPIB_REN_ASSERT_ADDRESS      3   "Assert REN line and address device."
        VI_GPIB_REN_ASSERT_LLO          4   "Send LLO to any devices that are addressed to listen."
        VI_GPIB_REN_ASSERT_ADDRESS_LLO  5   "Address this device and send it LLO, putting it in RWLS."
        VI_GPIB_REN_ADDRESS_GTL         6   "Send the Go To Local command (GTL) to this device."
    }
}
