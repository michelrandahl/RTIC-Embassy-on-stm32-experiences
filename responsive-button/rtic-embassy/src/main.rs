#![no_main]
#![no_std]

use embassy_stm32::gpio::{Level,Output,Speed,Input,Pull};
use embassy_stm32::time::Hertz;
use rtic_monotonics::systick::prelude::*;
use {defmt_rtt as _, panic_probe as _};
use cortex_m::peripheral::NVIC;

// Define the tick rate for the system timer
pub const TICK_RATE_HZ : u32 = 1_000;

// Set up the system timer as a monotonic timer for RTIC
systick_monotonic!(Mono, TICK_RATE_HZ);

// Import necessary modules from the embassy_stm32 crate into a `pac` module to be used with RTIC
pub mod pac {
    pub use embassy_stm32::pac::Interrupt as interrupt;
    pub use embassy_stm32::pac::*;
    pub use embassy_stm32::Peripherals;
}

// Helper function to turn LED off (active-low LED)
fn turn_led_off<T : embassy_stm32::gpio::Pin>(led : &mut Output<T>) {
    led.set_high()
}

// Helper function to turn LED on (active-low LED)
fn turn_led_on<T : embassy_stm32::gpio::Pin>(led : &mut Output<T>) {
    led.set_low()
}

fn setup_external_interrupt_for_user_button_1() {
    // EXTI: External Interrupt/Event Controller
    // SYSCFG: System Configuration Controller
    // NVIC: Nested Vectored Interrupt Controller
    // user-button-1 on the f303 nucleo-board is PC13, its external interrupt line is in EXTI15_10.
    // pin numbers map directly to same number for external interrupts, eg PC13 is EXTI13.

    // Connect EXTI13 to PC13
    // PC13 is on port C, which is represented by 0b010
    // This step is necessary to map the physical pin to the correct EXTI line
    pac::SYSCFG.exticr(3).write(|w| w.set_exti(13 % 4, 0b010) );

    // Unmask (enable) the interrupt on EXTI line 13
    // This allows the interrupt to be triggered by the button press
    pac::EXTI.imr(0).write(|w| w.set_line(13, true));

    // Set interrupt trigger to falling edge (button press)
    // This causes an interrupt when the button is pressed (voltage goes from high to low)
    pac::EXTI.ftsr(0).write(|w| w.set_line(13, true));

    // Set interrupt trigger to rising edge (button release)
    // This causes an interrupt when the button is released (voltage goes from low to high)
    pac::EXTI.rtsr(0).write(|w| w.set_line(13, true));

    // Enable the EXTI15_10 interrupt in the NVIC
    // EXTI15_10 covers external interrupt lines 10 to 15, including our button on line 13
    // This step is crucial as it allows the CPU to respond to this interrupt
    unsafe { NVIC::unmask(pac::Interrupt::EXTI15_10) };
}

// Function to clear the pending interrupt flag for user button 1
fn clear_pending_interrupt_register_for_user_button_1 (){
    // Clear the pending interrupt flag for EXTI line 13
    // This is necessary to acknowledge the interrupt and allow future interrupts
    // If not cleared, the interrupt would keep firing repeatedly
    pac::EXTI.pr(0).write(|w| w.set_line(13, true));
}


#[rtic::app(device = pac, peripherals = true, dispatchers = [SPI1])]
mod app {
    use embassy_stm32::{peripherals::PC13, Config};
    use rtic_sync::{channel::*, make_channel};
    use embassy_futures::select;
    use super::*;

    // Define clock speed and channel size
    const CLOCK_SPEED : Hertz = Hertz::mhz(16);
    const CHANNEL_SIZE: usize = 3;

    #[shared]
    struct Shared { }

    #[local]
    struct Local {
        button       : Input<'static, PC13>,
        led          : Output<'static, embassy_stm32::peripherals::PB7>,
        chan_send    : Sender<'static, bool, CHANNEL_SIZE>,
    }

    #[init]
    fn init(ctx : init::Context) -> (Shared, Local) {
        defmt::println!("Hello World!");
        defmt::println!("clock speed {}", CLOCK_SPEED.0);

        // Start the monotonic timer, to be used for delay timeouts
        Mono::start(ctx.core.SYST, CLOCK_SPEED.0);

        let mut conf : Config = Default::default();
        conf.rcc.sysclk = Some(CLOCK_SPEED);
        let p = embassy_stm32::init(conf);

        // On nucleo-f303 PB7 is `LD2`
        let mut led = Output::new(p.PB7, Level::High, Speed::Low);

        setup_external_interrupt_for_user_button_1();

        // On nucleo-f303 PC13 is `B1 User`
        // NOTE: button must be pull-down for the nucleo F303 board
        let button = Input::new(p.PC13, Pull::Down);
        let (chan_send, chan_rec) = make_channel!(bool, CHANNEL_SIZE);

        turn_led_off(&mut led);

        // Spawn the forever blinking task
        blink::spawn(chan_rec).ok();

        (Shared {}, Local {button,led,chan_send})
    }

    #[idle]
    fn idle(_ : idle::Context) -> ! {
        defmt::println!("idle");
        loop {
            // WFI: Wait For Interrupt
            // This instruction puts the processor into a low-power sleep state
            // The processor will wake up when an interrupt occurs
            cortex_m::asm::wfi();

            // Consequences of using WFI:
            // Positive:
            // 1. Reduces power consumption when the system is idle
            // 2. Allows for quick response to interrupts (like button presses)
            // 3. Simplifies the main loop - no need for busy-waiting or complex polling

            // Potential Negatives:
            // 1. If not all interrupt sources are properly set up, the system might
            //    become unresponsive or miss events
            // 2. In some cases, very frequent interrupts might negate power savings

            // Note: The exact behavior and power savings of WFI can vary depending on
            // the specific microcontroller and its configuration
        }
    }

    // hardware-task, Interrupt handler for button press (EXTI15_10)
    #[task(binds = EXTI15_10, priority = 2, local = [button,chan_send])]
    fn button_press(ctx : button_press::Context) {
        clear_pending_interrupt_register_for_user_button_1();

        defmt::println!("button_press");

        let button = ctx.local.button;

        ctx.local.chan_send.try_send(button.is_high()).ok();
    }

    // software-task, forever Blinking task
    #[task(priority = 1, local = [led])]
    async fn blink(ctx : blink::Context, mut chan_rec : Receiver<'static, bool, CHANNEL_SIZE>) {
        let led = ctx.local.led;
        let mut led_on = false;

        let now = Mono::now();
        let mut last_toggle = now;
        // setting initial blink rate to two seconds
        let mut blink_rate_delay = 2000;
        let mut current_time = now;

        loop {
            let delay_time = blink_rate_delay.millis();

            // Check if it's time to toggle the LED
            if current_time.ticks() - last_toggle.ticks() >= delay_time.ticks() {
                last_toggle = current_time;
                led_on = !led_on;
                if led_on {
                    turn_led_on(led);
                    defmt::println!("led on");
                } else {
                    turn_led_off(led);
                    defmt::println!("led off");
                }
            }

            // Wait for either a button press or the next toggle time
            // NOTE: usage of embassy-futures select function `Wait for one of two futures to complete.`
            match select::select(
                chan_rec.recv(),
                // NOTE: We delay until a specific timestamp instead of simply using `Mono::delay(...)`.
                // If we used `delay` then the actual time for the loop, would be processing-time plus delay.
                Mono::delay_until(current_time + delay_time)
            ).await {
                select::Either::First(Ok(button_pressed)) => {
                    // Adjust blink rate based on button state
                    if button_pressed {
                        blink_rate_delay = 50; // Fast blink when button is pressed
                    } else {
                        blink_rate_delay = 2000; // Slow blink when button is released
                    }
                    current_time = Mono::now();
                }
                select::Either::First(Err(_)) => {
                    current_time = Mono::now();
                }
                select::Either::Second(_) => {
                    // Update current time after delay.
                    // Adding to current_time, instead of getting Mono::now() and adding delay,
                    // prevents possible time drift.
                    current_time += delay_time;
                }
            }
        }
    }
}
