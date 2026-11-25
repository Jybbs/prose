def press_until_moved(device, presses):
    start = device.position, device.tiles
    for button in presses:
        device.press(button)
    if (device.position, device.tiles) == start:
        return False
    return True
