# Batty - Battery Health Tool for Linux

### What this is for
- Batty is meant to be installed and used in tandem with [power-profiles-daemon](https://gitlab.freedesktop.org/upower/power-profiles-daemon)
- Do not use this with [TLP](https://github.com/linrunner/TLP) as it can cause unpredictable behavior. Usually TLP can solve this however for projects like [Omarchy](https://github.com/basecamp/omarchy) where TLP is not provided, Batty can work in substitute, which inspired me to build this simple tool.
- Can use the TUI to alter battery threshold

### How to use it

#### Install Batty

```bash
cargo install batty
```
---

After installation, batty is placed in ~/.cargo/bin. To run it directly, ensure ~/.cargo/bin is in your $PATH. Add it to your shell configuration (e.g., ~/.bashrc or ~/.zshrc):

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

Then reload your shell:

```bash
source ~/.bashrc  # or ~/.zshrc
``` 

#### If ~/.cargo/bin is in your $PATH, you can just run:

```bash
batty
```

To run batty, it requires root privileges:

#### Option A - Use CLI

```bash
sudo ~/.cargo/bin/batty --value 80
```

Works immediately. Keep in mind it is not persistent yet.

---

#### Option B - Use TUI

```bash
sudo ~/.cargo/bin/batty --tui
```

This will give you write access in the TUI
