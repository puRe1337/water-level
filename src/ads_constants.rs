// ADS1115 I2C address when ADDR pin pulled to ground
pub const ADDR_ADS115:     u16 = 0x48; // Address of first ADS1115 chip  - i2cdetect -y 1 should print 48

// ADS1115 register addresses.
pub const REG_CONFIGURATION: u8 = 0x01;
pub const REG_CONVERSION:    u8 = 0x00;