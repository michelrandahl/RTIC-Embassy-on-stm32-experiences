# RTIC+Embassy on stm32: Experiences and Notes
This repo serves as a reference for future-me on how to work with RTIC on STM32.
While the focus will be on using RTIC, there will also be examples using pure Embassy.

# What's RTIC?
It's a Rust framework for embedded programming. It helps the programmer with being explicit and organized about state and priorities.
From RTIC's developers point of view; RTIC is a hardware accelerated RTOS that utilizes the hardware such as the NVIC on Cortex-M MCUs, CLIC on RISC-V etc. to perform scheduling, rather than a more classical software OS kernel.
[RTIC-docs](https://rtic.rs/2/book/en/preface.html)

RTIC is a Rust framework for embedded programming.
It helps the programmer be explicit and organized about state and priorities.
From a developers' point of view, RTIC is a hardware-accelerated RTOS that utilizes the hardware such as the NVIC on Cortex-M MCUs, CLIC on RISC-V, etc., to perform scheduling, rather than a more classical software OS kernel.
For more information, see the [RTIC docs](https://rtic.rs/2/book/en/preface.html).

## RTIC is a Tiny Bring-Your-Own-HAL Framework
RTIC is very small and focused; it does not come with any HALs (Hardware Abstraction Layers) or PACs (Peripheral Access Crates).
Instead, it allows the programmer to plug in their own choice of HAL and PAC.

### Choosing a HAL for RTIC
Finding an appropriate STM32 HAL to use with RTIC can be a challenge.
If you want to use the touch capabilities of STM32 (TSC - Touch Sensing Controller) or DMA (Direct Memory Access) capabilities, then you will need to find a HAL that already implements this, or worst case, you will have to fork and implement it yourself.

#### Individual 'Free-Roaming' HALs
You can choose to use one of the individual STM32 HALs such as [stm32f3xx-hal](https://github.com/stm32-rs/stm32f3xx-hal) or [stm32l0xx-hal](https://github.com/stm32-rs/stm32l0xx-hal).
However, be aware that these HALs tend to implement things in different opinionated ways.
If you, for example, fork and implement TSC support for one HAL and then change your mind about which STM32 MCU you want to use, then you might have to re-implement TSC all over again for the new HAL, unable to reuse your old solution.

#### [STM32-HAL2](https://crates.io/crates/stm32-hal2): A Promise of a More Unified HAL
STM32-HAL2 aims to provide a more unified approach, covering more STM32 HALs in one crate.
This is great if you don't enjoy implementing the same things over and over again just because you decide to change STM32 units.
However, this crate does not seem to support L0-based STM32 HALs, and I was personally unable to get a simple blink example running for the F303 MCU.

The repo has recent activity and lists 16 contributors, so it might improve in the future.

#### [Embassy-stm32](https://crates.io/crates/embassy-stm32) crate: A Very Unified HAL with Lots of Momentum
Another option is to leverage the work from the Embassy project.
The embassy-stm32 HAL aims to provide a safe, idiomatic hardware abstraction layer for all STM32 families.
The HAL implements both blocking and async APIs for many peripherals.

The embassy-stm32 repo has recent activity and lists 348 contributors, indicating a lot of momentum and many pairs of developer eyes to correct errors.

##### Downsides to using Embassy-stm32 with RTIC
When using the embassy-stm32 crate with RTIC, you might have to disable specific features such as "exti" (external interrupt support) since it contains definitions that will clash with RTIC definitions.
For these disabled features, you will have to resort to low (PAC)-level configuration for external interrupts (see the responsive-button RTIC solution in this repo for an example).

# What's Embassy?
Embassy is an embedded async framework for Rust that leverages the async/await syntax for writing asynchronous code.
It is designed to provide efficient and safe concurrency on embedded systems without requiring an RTOS.
It does however not provide the same explicit and organized approach to state and priority handling that RTIC does, and the footprint of the framework is much bigger according to some developers.
For more information, see the [Embassy docs](https://embassy.dev/).

## Batteries included: A Unified HAL Approach
Embassy provides a very friendly and well-rounded API and auxiliary crates for many tasks.
Embassy has associated HAL implementations for many types of MCUs (Microcontroller Units).
For the context of this repo, there is the `embassy-stm32` crate, which contains PAC and HAL definitions that support a long list of STM32 devices.
