# Batty - Battery Threshold CLI

### What this is for
- Batty is meant to be installed and used in tandem with [power-profiles-daemon](https://gitlab.freedesktop.org/upower/power-profiles-daemon)
- Do not use this with [TLP](https://github.com/linrunner/TLP) as it can cause unpredictable behavior. Usually TLP can solve this however for projects like [Omarchy](https://github.com/basecamp/omarchy) where TLP is not provided, Batty can work in substitute, which inspired me to build this simple tool.

### How to use it

#### Install Batty

```bash
cargo install batty
```
---

#### Option A — Use full path

```bash
sudo ~/.cargo/bin/batty --value 80
```

Works immediately.

---

#### Option B — Pass your user PATH to sudo

```bash
sudo env "PATH=$PATH" batty --value 80
```

`env "PATH=$PATH"` temporarily tells `sudo` to use your user’s PATH.

---

#### Option C — Install system-wide

```bash
sudo cp ~/.cargo/bin/batty /usr/local/bin/
sudo chmod +x /usr/local/bin/batty
```

Now `sudo batty --value 80` works without specifying the full path.
