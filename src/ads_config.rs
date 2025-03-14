use bitflags::bitflags;

// https://cdn-shop.adafruit.com/datasheets/ads1115.pdf
// Konfiguration für Single-Shot Messung auf A0
// 0x83, 0xE3: 
// - Bit 15: Start single-shot conversion
// - Bits 14-12: Input multiplexer to A0
// - Bits 11-9: Gain = 1 (+/- 4.096V)
// - Bit 8: Device operating mode  mode (1) or continuous mode (0)
// - Bits 7-5: Data Rate 128 SPS default
// - Bit 4: COMP_MODE: Comparator mode
// - Bit 3: COMP_POL: Comparator polarity
// - Bit 2: Non-latching comparator
// - Bits 1-0: COMP_QUE: Comparator queue and disable
// let first8bit = 0b10000011; // 0x83
// let second8bit = 0b11100011; // 0xE3
bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct AdsConfig: u16 {
        // Bit 15 - Operational Status / Single-shot conversion start
        const START_CONV     = 0b1000_0000_0000_0000;

        // Bits 14:12 - Input multiplexer
        const MUX_AIN0_AIN1 = 0b0000_0000_0000_0000; // 000: Default
        const MUX_AIN0_AIN3 = 0b0001_0000_0000_0000; // 001
        const MUX_AIN1_AIN3 = 0b0010_0000_0000_0000; // 010
        const MUX_AIN2_AIN3 = 0b0011_0000_0000_0000; // 011
        const MUX_AIN0_GND  = 0b0100_0000_0000_0000; // 100
        const MUX_AIN1_GND  = 0b0101_0000_0000_0000; // 101
        const MUX_AIN2_GND  = 0b0110_0000_0000_0000; // 110
        const MUX_AIN3_GND  = 0b0111_0000_0000_0000; // 111

        // Bits 11:9 - Programmable gain amplifier
        const GAIN_6_144V   = 0b0000_0000_0000_0000; // 000: ±6.144V
        const GAIN_4_096V   = 0b0000_0010_0000_0000; // 001: ±4.096V
        const GAIN_2_048V   = 0b0000_0100_0000_0000; // 010: ±2.048V
        const GAIN_1_024V   = 0b0000_0110_0000_0000; // 011: ±1.024V
        const GAIN_0_512V   = 0b0000_1000_0000_0000; // 100: ±0.512V
        const GAIN_0_256V_1 = 0b0000_1010_0000_0000; // 101: ±0.256V
        const GAIN_0_256V_2 = 0b0000_1100_0000_0000; // 110: ±0.256V
        const GAIN_0_256V_3 = 0b0000_1110_0000_0000; // 111: ±0.256V

        // Device operating mode (Bit 8)
        const MODE_CONTINUOUS = 0b0000_0000_0000_0000; // 0: Continuous conversion mode
        const MODE_SINGLE    = 0b0000_0001_0000_0000; // 1: Power-down single-shot mode (default)

        // Data rate (Bits 7:5)
        const DR_8SPS       = 0b0000_0000_0000_0000; // 000: 8 SPS
        const DR_16SPS      = 0b0000_0000_0010_0000; // 001: 16 SPS
        const DR_32SPS      = 0b0000_0000_0100_0000; // 010: 32 SPS
        const DR_64SPS      = 0b0000_0000_0110_0000; // 011: 64 SPS
        const DR_128SPS     = 0b0000_0000_1000_0000; // 100: 128 SPS (default)
        const DR_250SPS     = 0b0000_0000_1010_0000; // 101: 250 SPS
        const DR_475SPS     = 0b0000_0000_1100_0000; // 110: 475 SPS
        const DR_860SPS     = 0b0000_0000_1110_0000; // 111: 860 SPS

        // Comparator mode (Bit 4)
        const COMP_MODE_TRADITIONAL = 0b0000_0000_0000_0000; // 0: Traditional comparator (default)
        const COMP_MODE_WINDOW     = 0b0000_0000_0001_0000; // 1: Window comparator

        // Comparator polarity (Bit 3)
        const COMP_POL_ACTIVE_LOW  = 0b0000_0000_0000_0000; // 0: Active low (default)
        const COMP_POL_ACTIVE_HIGH = 0b0000_0000_0000_1000; // 1: Active high

        // Latching comparator (Bit 2)
        const COMP_LAT_NONLATCH = 0b0000_0000_0000_0000; // 0: Non-latching (default)
        const COMP_LAT_LATCH    = 0b0000_0000_0000_0100; // 1: Latching

        // Comparator queue and disable (Bits 1:0)
        const COMP_QUE_ASSERT_1 = 0b0000_0000_0000_0000; // 00: Assert after one conversion
        const COMP_QUE_ASSERT_2 = 0b0000_0000_0000_0001; // 01: Assert after two conversions
        const COMP_QUE_ASSERT_4 = 0b0000_0000_0000_0010; // 10: Assert after four conversions
        const COMP_QUE_DISABLE  = 0b0000_0000_0000_0011; // 11: Disable comparator (default)
    }
}