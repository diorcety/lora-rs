#[cfg(feature = "defmt")]
use defmt::trace;
#[cfg(feature = "log")]
use log::trace;
#[cfg(all(not(feature = "defmt"), not(feature = "log")))]
macro_rules! trace {
    ($($arg:tt)*) => {};
}

use embedded_hal_async::spi::{Operation, SpiDevice};

use crate::mod_params::RadioError::{self, SPI};
use crate::mod_traits::InterfaceVariant;

pub struct SpiInterface<SPI, IV> {
    pub spi: SPI,
    pub iv: IV,
}

pub struct HexSlice<'a>(pub &'a [u8]);

#[cfg(feature = "defmt")]
impl<'a> defmt::Format for HexSlice<'a> {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "[");
        for (i, byte) in self.0.iter().enumerate() {
            if i > 0 {
                defmt::write!(f, ", ");
            }
            defmt::write!(f, "{:02x}", byte);
        }
        defmt::write!(f, "]");
    }
}

#[cfg(feature = "log")]
impl<'a> core::fmt::Display for HexSlice<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "[")?;
        for (i, byte) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{:02x}", byte)?;
        }
        write!(f, "]")
    }
}

impl<SPI, IV> SpiInterface<SPI, IV>
where
    SPI: SpiDevice<u8>,
    IV: InterfaceVariant,
{
    pub fn new(spi: SPI, iv: IV) -> Self {
        Self { spi, iv }
    }

    // Write a buffer to the radio.
    pub fn write<'a>(
        &'a mut self,
        write_buffer: &'a [u8],
        is_sleep_command: bool,
    ) -> impl core::future::Future<Output = Result<(), RadioError>> + 'a {
        #[inline(never)]
        async move {
            self.spi.write(write_buffer).await.map_err(|_| SPI)?;
            trace!("write: {}", HexSlice(write_buffer));

            if !is_sleep_command {
                self.iv.wait_on_busy().await?;
            }

            Ok(())
        }
    }

    // Write
    pub fn write_with_payload<'a>(
        &'a mut self,
        write_buffer: &'a [u8],
        payload: &'a [u8],
        is_sleep_command: bool,
    ) -> impl core::future::Future<Output = Result<(), RadioError>> + 'a {
        #[inline(never)]
        async move {
            let mut ops = [Operation::Write(write_buffer), Operation::Write(payload)];
            self.spi.transaction(&mut ops).await.map_err(|_| SPI)?;
            trace!("write_buf: {} -> {}", HexSlice(write_buffer), HexSlice(payload));

            if !is_sleep_command {
                self.iv.wait_on_busy().await?;
            }

            Ok(())
        }
    }

    // Request a read, filling the provided buffer.
    pub fn read<'a>(
        &'a mut self,
        write_buffer: &'a [u8],
        read_buffer: &'a mut [u8],
    ) -> impl core::future::Future<Output = Result<(), RadioError>> + 'a {
        #[inline(never)]
        async move {
            {
                let mut ops = [Operation::Write(write_buffer), Operation::Read(read_buffer)];

                self.spi.transaction(&mut ops).await.map_err(|_| SPI)?;
            }

            self.iv.wait_on_busy().await?;

            trace!(
                "read: addr={}, len={}, data={}",
                HexSlice(write_buffer),
                read_buffer.len(),
                HexSlice(read_buffer)
            );

            Ok(())
        }
    }

    // Request a read with status, filling the provided buffer and returning the status.
    pub fn read_with_status<'a>(
        &'a mut self,
        write_buffer: &'a [u8],
        read_buffer: &'a mut [u8],
    ) -> impl core::future::Future<Output = Result<u8, RadioError>> + 'a {
        #[inline(never)]
        async move {
            let mut status = [0u8];
            {
                let mut ops = [
                    Operation::Write(write_buffer),
                    Operation::Read(&mut status),
                    Operation::Read(read_buffer),
                ];

                self.spi.transaction(&mut ops).await.map_err(|_| SPI)?;
            }

            self.iv.wait_on_busy().await?;

            trace!(
                "read: addr={}, len={}, status={:02x}, buf={}",
                HexSlice(write_buffer),
                read_buffer.len(),
                status[0],
                HexSlice(read_buffer)
            );

            Ok(status[0])
        }
    }
}
