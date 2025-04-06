#![no_std]
#![no_main]

use core::fmt::Write;

use defmt::*;
use {defmt_rtt as _, panic_probe as _};
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::usart::{self, Uart};
use embassy_stm32::{bind_interrupts, peripherals};
use embassy_time::Timer;
use heapless::String;

bind_interrupts!(struct NoteIrqs {
    USART2 => usart::InterruptHandler<peripherals::USART2>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // Configure and initialize system clock tree
    let mut config = embassy_stm32::Config::default();
    {
        use embassy_stm32::rcc::*;
        config.rcc.sys = Sysclk::PLL1_R;
        config.rcc.hsi = true;
        config.rcc.pll = Some(Pll {
            source: PllSource::HSI,
            prediv: PllPreDiv::DIV1,
            mul: PllMul::MUL20,
            divp: None,
            divq: None,
            divr: Some(PllRDiv::DIV4),
        });
        config.rcc.ls = LsConfig::default_lse();
        config.rcc.mux.usart1sel = mux::Usart1sel::SYS;
    }
    let p = embassy_stm32::init(config);

    info!("Hello World!");

    let mut config = usart::Config::default();
    config.baudrate = 115200;
    let mut usart = Uart::new(
        p.USART2, p.PA3, p.PA2, NoteIrqs, p.DMA1_CH7, p.DMA1_CH6, config,
    )
    .unwrap();

    let _aux_en = Output::new(p.PB2, Level::High, Speed::Low);
    Timer::after_millis(100).await;

    for _n in 0u32.. {
        let mut s: String<128> = String::new();
        core::write!(&mut s, "\n").unwrap();

        info!("Writing...");
        usart.write(s.as_bytes()).await.ok();

        info!("wrote DMA");
        info!("Reading...");
        let mut buffer = [0_u8; 256];
        let r = usart.read_until_idle(&mut buffer).await.ok();
        if let Some(len) = r {
            info!(
                "Received {} bytes: \"{:?}\"",
                len,
                core::str::from_utf8(&buffer[0..len]).unwrap()
            );
        } else {
            info!("Did not receive anything...");
        }

        Timer::after_millis(1000).await;
    }
}
