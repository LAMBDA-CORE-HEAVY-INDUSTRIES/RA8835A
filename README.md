# RA8835A

Driver for RA8835A / SED1335 display controllers.

## Examples

In order to keep the driver code generic, it is up to the user to implement their own parallel bus implementation and functionality to toggle between input/output modes. See `examples/` for example implementation with STM32F411 using `DynamicPin`.
