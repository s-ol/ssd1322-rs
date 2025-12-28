//! This module provides shims for the `embedded-hal` hardware corresponding to the SSD1322's
//! supported electrical/bus interfaces. It is a shim between `embedded-hal` implementations and
//! the display driver's command layer.

/// An interface for the SSD1322 implements this trait, which provides the basic operations for
/// sending pre-encoded commands and data to the chip via the interface.
#[maybe_async_cfg::maybe(sync(keep_self), async(feature = "async"))]
pub trait DisplayInterface {
    type Error;

    async fn send_command(&mut self, cmd: u8) -> Result<(), Self::Error>;
    async fn send_data(&mut self, buf: &[u8]) -> Result<(), Self::Error>;
}

pub mod spi {
    //! The SPI interface supports the "4-wire" interface of the driver, such that each word on the
    //! SPI bus is 8 bits. The "3-wire" mode is not supported, as it replaces the D/C GPIO with a
    //! 9th bit on each SPI word, and `embedded-hal` SPI traits do not currently support
    //! non-byte-aligned SPI word lengths.

    use embedded_hal as hal;
    #[cfg(feature = "async")]
    use embedded_hal_async as hal_async;

    use super::DisplayInterface;
    #[cfg(feature = "async")]
    use super::DisplayInterfaceAsync;

    /// The union of all errors that may occur on the SPI interface. This consists of variants for
    /// the error types of the D/C GPIO and the SPI bus.
    #[derive(Debug)]
    pub enum SpiInterfaceError<DCE, SPIE> {
        DCError(DCE),
        SPIError(SPIE),
    }

    impl<DCE, SPIE> SpiInterfaceError<DCE, SPIE> {
        fn from_dc(e: DCE) -> Self {
            Self::DCError(e)
        }
        fn from_spi(e: SPIE) -> Self {
            Self::SPIError(e)
        }
    }

    /// A configured `DisplayInterface` for controlling an SSD1322 via 4-wire SPI.
    pub struct SpiInterface<SPI, DC> {
        /// The SPI master device connected to the SSD1322.
        spi: SPI,
        /// A GPIO output pin connected to the D/C (data/command) pin of the SSD1322 (the fourth
        /// "wire" of "4-wire" mode).
        dc: DC,
    }

    impl<SPI, DC> SpiInterface<SPI, DC>
    where
        SPI: hal::spi::SpiDevice,
        DC: hal::digital::OutputPin,
    {
        /// Create a new SPI interface to communicate with the display driver. `spi` is the SPI
        /// master device, and `dc` is the GPIO output pin connected to the D/C pin of the SSD1322.
        pub fn new(spi: SPI, dc: DC) -> Self {
            Self { spi, dc }
        }
    }

    impl<SPI, DC> DisplayInterface for SpiInterface<SPI, DC>
    where
        SPI: hal::spi::SpiDevice,
        DC: hal::digital::OutputPin,
    {
        type Error = SpiInterfaceError<DC::Error, SPI::Error>;

        /// Send a command word to the display's command register.
        fn send_command(&mut self, cmd: u8) -> Result<(), Self::Error> {
            self.dc.set_low().map_err(Self::Error::from_dc)?;
            self.spi.write(&[cmd]).map_err(Self::Error::from_spi)?;
            self.dc.set_high().map_err(Self::Error::from_dc)?;
            Ok(())
        }

        /// Send a sequence of data words to the display from a buffer.
        fn send_data(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
            self.spi.write(buf).map_err(Self::Error::from_spi)
        }
    }

    /// A configured async `DisplayInterface` for controlling an SSD1322 via 4-wire SPI.
    #[cfg(feature = "async")]
    pub struct SpiInterfaceAsync<SPI, DC> {
        /// The SPI master device connected to the SSD1322.
        spi: SPI,
        /// A GPIO output pin connected to the D/C (data/command) pin of the SSD1322 (the fourth
        /// "wire" of "4-wire" mode).
        dc: DC,
    }

    #[cfg(feature = "async")]
    impl<SPI, DC> SpiInterfaceAsync<SPI, DC>
    where
        SPI: hal_async::spi::SpiDevice,
        DC: hal::digital::OutputPin,
    {
        /// Create a new async SPI interface to communicate with the display driver. `spi` is the SPI
        /// master device, and `dc` is the GPIO output pin connected to the D/C pin of the SSD1322.
        pub fn new(spi: SPI, dc: DC) -> Self {
            Self { spi, dc }
        }
    }

    #[cfg(feature = "async")]
    impl<SPI, DC> DisplayInterfaceAsync for SpiInterfaceAsync<SPI, DC>
    where
        SPI: hal_async::spi::SpiDevice,
        DC: hal::digital::OutputPin,
    {
        type Error = SpiInterfaceError<DC::Error, SPI::Error>;

        /// Send a command word to the display's command register.
        async fn send_command(&mut self, cmd: u8) -> Result<(), Self::Error> {
            self.dc.set_low().map_err(Self::Error::from_dc)?;
            self.spi.write(&[cmd]).await.map_err(Self::Error::from_spi)?;
            self.dc.set_high().map_err(Self::Error::from_dc)?;
            Ok(())
        }

        /// Send a sequence of data words to the display from a buffer.
        async fn send_data(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
            self.spi.write(buf).await.map_err(Self::Error::from_spi)
        }
    }
}

#[cfg(test)]
pub mod test_spy {
    //! An interface for use in unit tests to spy on whatever was sent to it.

    use super::DisplayInterface;
    #[cfg(feature = "async")]
    use super::DisplayInterfaceAsync;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Clone, Debug, PartialEq)]
    pub enum Sent {
        Cmd(u8),
        Data(Vec<u8>),
    }

    pub struct TestSpyInterface {
        sent: Rc<RefCell<Vec<Sent>>>,
    }

    impl TestSpyInterface {
        pub fn new() -> Self {
            TestSpyInterface {
                sent: Rc::new(RefCell::new(Vec::new())),
            }
        }
        pub fn split(&self) -> Self {
            Self {
                sent: self.sent.clone(),
            }
        }
        pub fn check(&self, cmd: u8, data: &[u8]) {
            let sent = self.sent.borrow();
            if data.len() == 0 {
                assert_eq!(sent.len(), 1);
            } else {
                assert_eq!(sent.len(), 2);
                assert_eq!(sent[1], Sent::Data(data.to_vec()));
            }
            assert_eq!(sent[0], Sent::Cmd(cmd));
        }
        pub fn check_multi(&self, expect: &[Sent]) {
            assert_eq!(*self.sent.borrow(), expect);
        }
        pub fn clear(&mut self) {
            self.sent.borrow_mut().clear()
        }
    }

    impl DisplayInterface for TestSpyInterface {
        type Error = core::convert::Infallible;

        fn send_command(&mut self, cmd: u8) -> Result<(), Self::Error> {
            self.sent.borrow_mut().push(Sent::Cmd(cmd));
            Ok(())
        }
        fn send_data(&mut self, data: &[u8]) -> Result<(), Self::Error> {
            self.sent.borrow_mut().push(Sent::Data(data.to_vec()));
            Ok(())
        }
    }

    #[cfg(feature = "async")]
    impl DisplayInterfaceAsync for TestSpyInterface {
        type Error = core::convert::Infallible;

        async fn send_command(&mut self, cmd: u8) -> Result<(), Self::Error> {
            self.sent.borrow_mut().push(Sent::Cmd(cmd));
            Ok(())
        }
        async fn send_data(&mut self, data: &[u8]) -> Result<(), Self::Error> {
            self.sent.borrow_mut().push(Sent::Data(data.to_vec()));
            Ok(())
        }
    }
}
