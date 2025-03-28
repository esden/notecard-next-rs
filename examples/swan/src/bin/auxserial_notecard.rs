#![no_std]
#![no_main]

use blues_notecard_next::Notecard;

use defmt::*;
use {defmt_rtt as _, panic_probe as _};
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::usart::{self, BufferedUart};
use embassy_stm32::{bind_interrupts, peripherals};
use embassy_time::{Delay, Instant, Timer};

bind_interrupts!(struct NoteIrqs {
    USART2 => usart::BufferedInterruptHandler<peripherals::USART2>;
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
    let mut tx_buffer = [0_u8; 256];
    let mut rx_buffer = [0_u8; 256];
    let usart = BufferedUart::new(
        p.USART2,
        NoteIrqs,
        p.PA3,
        p.PA2,
        &mut tx_buffer,
        &mut rx_buffer,
        config,
    )
    .unwrap();

    // Enable aux serial
    let _aux_en = Output::new(p.PB2, Level::High, Speed::Low);
    Timer::after_millis(100).await;

    struct EmbassyClock;
    impl blues_notecard_next::Now for EmbassyClock {
        fn now_micros(&self) -> u64 {
            Instant::now().as_micros()
        }
    }

    let clock = EmbassyClock;
    let delay = Delay;

    // Configure notecard
    let mut note = Notecard::new(usart, clock, delay);

    let r = note.transaction().await;

    info!("done {}", r);

    loop {}
}
