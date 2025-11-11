import init, { update_rom_data } from "./bin/gametank-emu-rs.js";

// Fetch binary ROM data from API
async function fetchAndLoadROM() {
    try {
        const response = await fetch("http://localhost:41123/api/games/e3e3b8e6-a8a1-4ba4-a577-eb98f716cc11");
        if (!response.ok) throw new Error(`HTTP error! Status: ${response.status}`);

        const json = await response.json(); // Parse JSON
        if (!json.game_rom || !Array.isArray(json.game_rom)) {
            throw new Error("Invalid ROM data received");
        }

        const uint8Array = new Uint8Array(json.game_rom); // Convert JSON number array to Uint8Array

        update_rom_data(uint8Array); // Send ROM to Rust
        console.log(`Loaded ROM: ${uint8Array.length} bytes`);
    } catch (error) {
        console.error("Failed to load ROM:", error);
    }
}

window.fetchAndLoadROM = fetchAndLoadROM;

async function main() {
    await init();
}

main();
