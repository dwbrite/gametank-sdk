# GameTank Palette Resources

This directory contains capture-based palette files for the GameTank video hardware. These palettes map RGB color values to the GameTank's native HSL (Hue, Saturation, Luminosity) encoding system.

## Available Palette Files

- **`PALETTE.act`** - Adobe Color Table format (compatible with Aseprite, LibreSprite, Photoshop, and other Adobe-compatible software)
- **`PALETTE.gpl`** - GIMP Palette format (compatible with GIMP, Inkscape, and other software that supports GPL files)

Both files contain the same 256-color palette that represents the GameTank's color output as passed through a capture card. The palette is organized to map each byte value (0-255) to a color that closely matches what the GameTank video hardware would display for that byte value.

## Understanding the GameTank Color System

The GameTank video circuit is **not** palette-based in the traditional sense. Instead, each byte value encodes packed parameters for the video signal:

| Bitmask  | Physical Control          | Effective Output |
| -------- | ------------------------- | ---------------- |
| `11100000` | Color Carrier Phase Shift | Hue              |
| `00011000` | Color Carrier Amplitude   | Saturation       |
| `00000111` | DC Offset                 | Luminosity       |

The palette files provided here offer a convenient visual representation of what each byte value will look like when rendered, making it easier to create graphics that will display correctly on the GameTank hardware.

*Source: [Creating GameTank Graphics With Aseprite - GameTank Wiki](https://wiki.gametank.zone/doku.php?id=development:aseprite)*

## Loading the Palette

### Aseprite / LibreSprite

Aseprite and LibreSprite both support `.act` (Adobe Color Table) files natively:

1. **Create or open your image file**
   - Set Color Mode to "Indexed"
   - Recommended: Width 128px, Height at most 128px (GameTank constraints)

2. **Load the palette:**
   - Click on the **Palette** tab or window
   - Click the menu button (three horizontal lines) in the palette panel
   - Select **Load Palette** or **Open Palette**
   - Navigate to `resources/palettes/PALETTE.act`
   - Click **Open**

3. **Alternative method:**
   - File → Open → Select `PALETTE.act`
   - The palette will open as a new file
   - You can then copy colors or save it as your default palette

**Example workflow:**
```
1. File → New (128x128, Indexed color mode)
2. Palette menu → Load Palette → Select PALETTE.act
3. Draw your artwork using the palette colors
4. File → Export → Export Sprite Sheet (BMP format, 128px fixed width)
```

For detailed Aseprite workflow instructions, see: [Creating GameTank Graphics With Aseprite](https://wiki.gametank.zone/doku.php?id=development:aseprite)

### GIMP

GIMP supports `.gpl` (GIMP Palette) files:

1. **Open GIMP** and create or open your image

2. **Convert to indexed mode (if needed):**
   - Image → Mode → Indexed...
   - Choose "Use custom palette" and click the palette icon next to it
   - Or: Windows → Dockable Dialogs → Palettes

3. **Load the palette:**
   - Right-click in the Palettes dialog
   - Select **Import Palette** or **Open Palette**
   - Navigate to `resources/palettes/PALETTE.gpl`
   - Select it and click **Open**

4. **Apply the palette:**
   - If converting to indexed: Image → Mode → Indexed → Use custom palette → Select "GameTank Palette" (or the name you see)
   - The palette will appear in your Palettes dialog for easy access

**Example workflow:**
```
1. File → New (128x128 pixels)
2. Windows → Dockable Dialogs → Palettes
3. Right-click in Palettes → Import Palette → Select PALETTE.gpl
4. Image → Mode → Indexed → Use custom palette → Select the loaded palette
5. Draw your artwork
6. File → Export As → Save as BMP format
```

## Using the Palette in the SDK

The palette is integrated into the GameTank SDK's asset processing system. When you export BMP files from your graphics editor, the SDK automatically converts them to GameTank-compatible format.

### Best Practices

1. **Use palette colors directly** when possible for best color accuracy
2. **Limit color count** in sprite sheets to reduce memory usage (fewer colors = better bit-packing)
3. **Use magenta (255, 0, 255)** for transparent pixels
4. **Keep images within 128x128 pixels** for hardware compatibility
5. **Export as BMP** in indexed color mode for SDK compatibility

## Additional Resources

- **Official Wiki:** [Creating GameTank Graphics With Aseprite](https://wiki.gametank.zone/doku.php?id=development:aseprite)
- **Emulator Repository:** [GameTankEmulator - misc folder](https://github.com/clydeshaffer/GameTankEmulator/tree/main/misc)
- **SDK Documentation:** See `asset-macros/src/bmp.rs` for implementation details