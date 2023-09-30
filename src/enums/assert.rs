#![allow(overflowing_literals)]
#![allow(non_upper_case_globals)]

consts_to_enum! {
    #[format=dbg]
    #[repr(ViInt16)]
    /// This specifies how to assert the interrupt.
    /// 
    /// See [`assert_intr_signal`](crate::Instrument::assert_intr_signal)
    ///
    pub enum AssertIntrHow {
        VI_ASSERT_SIGNAL            -1           r#"Send the notification via a VXI signal."#
        VI_ASSERT_USE_ASSIGNED      0          r#"Use whatever notification method that has been assigned to the local device."#
        VI_ASSERT_IRQ1              1          r#"
        Send the interrupt via the specified VXI/VME IRQ line. This uses the standard VXI/VME ROAK (Release On AcKnowledge) interrupt mechanism, rather than the older VME RORA (Release On Register Access) mechanism.
        "#
        VI_ASSERT_IRQ2              2          r#"
        Send the interrupt via the specified VXI/VME IRQ line. This uses the standard VXI/VME ROAK (Release On AcKnowledge) interrupt mechanism, rather than the older VME RORA (Release On Register Access) mechanism.
        "#
        VI_ASSERT_IRQ3              3          r#"
        Send the interrupt via the specified VXI/VME IRQ line. This uses the standard VXI/VME ROAK (Release On AcKnowledge) interrupt mechanism, rather than the older VME RORA (Release On Register Access) mechanism.
        "#
        VI_ASSERT_IRQ4              4          r#"
        Send the interrupt via the specified VXI/VME IRQ line. This uses the standard VXI/VME ROAK (Release On AcKnowledge) interrupt mechanism, rather than the older VME RORA (Release On Register Access) mechanism.
        "#
        VI_ASSERT_IRQ5              5          r#"
        Send the interrupt via the specified VXI/VME IRQ line. This uses the standard VXI/VME ROAK (Release On AcKnowledge) interrupt mechanism, rather than the older VME RORA (Release On Register Access) mechanism.
        "#
        VI_ASSERT_IRQ6              6          r#"
        Send the interrupt via the specified VXI/VME IRQ line. This uses the standard VXI/VME ROAK (Release On AcKnowledge) interrupt mechanism, rather than the older VME RORA (Release On Register Access) mechanism.
        "#
        VI_ASSERT_IRQ7              7          r#"
        Send the interrupt via the specified VXI/VME IRQ line. This uses the standard VXI/VME ROAK (Release On AcKnowledge) interrupt mechanism, rather than the older VME RORA (Release On Register Access) mechanism.
        "#
    }
}

//pub const VI_TRIG_PROT_DEFAULT: u32 = 0;
//pub const VI_TRIG_PROT_ON: u32 = 1;
//pub const VI_TRIG_PROT_OFF: u32 = 2;
//pub const VI_TRIG_PROT_SYNC: u32 = 5;
//pub const VI_TRIG_PROT_RESERVE: u32 = 6;
//pub const VI_TRIG_PROT_UNRESERVE: u32 = 7;

consts_to_enum! {
    #[format=dbg]
    #[repr(ViUInt16)]
    /// Trigger protocol to use during assertion.
    /// * GPIB, Serial, TCPIP, USB
    /// 
    /// VI_TRIG_PROT_DEFAULT (0)
    ///
    /// * VXI
    ///
    /// VI_TRIG_PROT_DEFAULT (0),
    /// VI_TRIG_PROT_ON (1),
    /// VI_TRIG_PROT_OFF (2), and
    /// VI_TRIG_PROT_SYNC (5)
    ///
    /// * PXI
    ///
    /// VI_TRIG_PROT_RESERVE (6)
    /// VI_TRIG_PROT_UNRESERVE (7)
    /// 
    /// See [`assert_trigger`](crate::Instrument::assert_trigger)
    ///
    pub enum AssertTrigPro {
        VI_TRIG_PROT_DEFAULT    0
        VI_TRIG_PROT_ON         1
        VI_TRIG_PROT_OFF        2
        VI_TRIG_PROT_SYNC       5
        VI_TRIG_PROT_RESERVE    6
        VI_TRIG_PROT_UNRESERVE  7
    }
}

//pub const VI_UTIL_ASSERT_SYSRESET: u32 = 1;
//pub const VI_UTIL_ASSERT_SYSFAIL: u32 = 2;
//pub const VI_UTIL_DEASSERT_SYSFAIL: u32 = 3;

consts_to_enum! {
    #[format=dbg]
    #[repr(ViUInt16)]
    /// Specifies the utility bus signal to assert.
    /// 
    /// See [`assert_util_signal`](crate::Instrument::assert_util_signal)
    ///
    pub enum AssertBusSignal {
        VI_UTIL_ASSERT_SYSRESET    1
        VI_UTIL_ASSERT_SYSFAIL     2
        VI_UTIL_DEASSERT_SYSFAIL   3
    }
}