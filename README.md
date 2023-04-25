# ip-kvm

## Android

Android need stop adbd to release udc and mount configfs to `/sys/kernel/config`.

```bash
stop adbd
sudo mount -t configfs none /sys/kernel/config
sudo ./ip-kvm
```

## Linux

Linux need to load `libcomposite` to enable usb gadget.

```bash
sudo modprobe libcomposite
sudo ./ip-kvm
```