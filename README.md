# Keyboard Context

This is a tool so I have `ctrl:swapcaps` enabled in my settings in GNOME in my
laptop when it's moving around but disabled when my ErgoDox is connected to it
as in that case all the swap magic happens on the keyboard itself.

## How to use

Unless you have the same keyboard I have, you'll want to change the vendor and
product id to match yours. Then you can

```shell
% cargo build --release
% mkdir ~/bin
% cp target/release/keyboard-context ~/bin
% mkdir -p ~/.config/systemd/user
% cp systemd/keyboard-context.service ~/.config/systemd/user/
% systemctl --user enable keyboard-context
```
and then it should be running whenever you log in.
