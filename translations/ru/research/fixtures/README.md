# Происхождение платформенного fixture

virt-10.1.dtb — бинарное дерево устройств, сгенерированное QEMU 10.1.0, а не исходник другого ядра. Оно используется host parser tests; реальные запуски ядра читают DTB, предоставленный QEMU при загрузке.

## Повторная генерация

```text
qemu-system-aarch64 -machine virt-10.1,gic-version=3,virtualization=on,its=off,dtb-randomness=off,dumpdtb=target/kernel/virt.dtb -cpu cortex-a57 -accel tcg -smp 2 -m 256M -display none -monitor none -nic none
```

Копируйте сгенерированный файл только после сравнения платформенных дескрипторов и ревью изменения. Тест отклоняет каждое строгое усечение fixture, повреждённые offsets, version и alignment. Это подтверждает отклонение этих входов parser, а не универсальную безопасность boot firmware.
