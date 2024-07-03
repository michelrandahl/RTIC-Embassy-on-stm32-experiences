#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed, Input, Pull};
use embassy_time::{Duration, Instant, Timer};
use {defmt_rtt as _, panic_probe as _};
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::peripherals::{PC13, PB7};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel, mutex::Mutex};
use embassy_futures::select;

// Global variables for inter-task communication
static BUTTON_PRESSED: Mutex<CriticalSectionRawMutex, bool> = Mutex::new(false);
static BUTTON_CHANNEL: Channel<CriticalSectionRawMutex, bool, 3> = Channel::new();

// Task to handle button presses and releases
#[embassy_executor::task]
async fn button_task(mut button: ExtiInput<'static, PC13>) {
    println!("inside button task");
    loop {
        button.wait_for_rising_edge().await;
        println!("Button pressed!");
        *BUTTON_PRESSED.lock().await = true;
        BUTTON_CHANNEL.send(true).await;

        button.wait_for_falling_edge().await;
        println!("Button released!");
        *BUTTON_PRESSED.lock().await = false;
        BUTTON_CHANNEL.send(false).await;
    }
}

// Task to control LED blinking
#[embassy_executor::task]
async fn led_task(mut led: Output<'static, PB7>) {
    let mut led_on = false;
    let now = Instant::now();
    let mut last_toggle = now;

    // setting initial blink rate to two seconds
    let mut blink_rate_delay = Duration::from_millis(2000);
    let mut current_time = now;

    loop {
        if current_time - last_toggle >= blink_rate_delay {
            last_toggle = current_time;
            led_on = !led_on;
            if led_on {
                led.set_high();
                defmt::println!("led on");
            } else {
                led.set_low();
                defmt::println!("led off");
            }
        }

        // Wait for either a button press or the next toggle time
        // NOTE: usage of embassy-futures select function `Wait for one of two futures to complete.`
        match select::select(
            BUTTON_CHANNEL.receive(),
            // NOTE: We delay until a specific timestamp instead of simply using `Timer::after(..)` (regular delay).
            // If we used a regular `delay` then the actual time for the loop, would be processing-time plus delay.
            Timer::at(current_time + blink_rate_delay)
        ).await {
            select::Either::First(button_pressed) => {
                blink_rate_delay = if button_pressed {
                    Duration::from_millis(50) // Fast blink when button is pressed
                } else {
                    Duration::from_millis(2000) // Slow blink when button is released
                };
                current_time = Instant::now();
            }
            select::Either::Second(_) => {
                // Update current time after delay.
                // Adding to current_time, instead of getting Instant::now() and adding delay,
                // prevents possible time drift.
                current_time += blink_rate_delay;
            }
        };
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    println!("Hello World!");

    let led = Output::new(p.PB7, Level::High, Speed::Low);

    // NOTE: button must be configured as pull-down for the nucleo F303 board
    let button = Input::new(p.PC13, Pull::Down);
    // Convert the button input to an ExtiInput for interrupt-style operation
    let button = ExtiInput::new(button, p.EXTI13);

    // Spawn the button task to handle interrupts
    // NOTE: Embassy will set up proper interrupt config behind the scenes for `EXTI15_10`
    spawner.spawn(button_task(button)).unwrap();

    // Spawn the LED task
    spawner.spawn(led_task(led)).unwrap();

    // The main function ends here, but the Embassy runtime continues to run
    // It will automatically enter a low-power mode when no tasks are active
    // This replaces the explicit WFI in the RTIC idle task
}
