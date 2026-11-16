local f = CreateFrame("Frame")
f:RegisterEvent("PLAYER_ENTERING_WORLD")
f:SetScript("OnEvent", function()
    pcall(SetCVar, "advancedCombatLogging", 1)
    LoggingCombat(true)
    LoggingChat(true)
    print("Corvid: combat", LoggingCombat(), "chat", LoggingChat(),
          "advanced", GetCVar("advancedCombatLogging"))
end)

SLASH_CORVID1 = "/corvid"
SlashCmdList.CORVID = function()
    for _, unit in ipairs({"player", "party1", "party2"}) do
        local map = C_Map.GetBestMapForUnit(unit)
        local pos = map and C_Map.GetPlayerMapPosition(map, unit)
        if pos then
            local x, y = pos:GetXY()
            print(unit, map, string.format("%.3f, %.3f", x, y))
        else
            print(unit, "no position")
        end
    end
end

-- Live position strip
--
-- Addons can't write files while the game runs, so Corvid paints where you
-- and your party are into rows of small squares in the top-left corner, and
-- the desktop app reads them off the screen. Each square holds 3 bits, one per
-- color channel, and a channel is only ever fully on or off, so gamma and
-- brightness can't change it. Numbers are highest bit first, x and y are 0-1
-- scaled to 0-65535, and each row's checksum is 255 minus the sum of its other
-- bytes, mod 256.
--
-- Your row, 21 squares:
--   1, 2   magenta then green, so the app can tell the strip is there
--   3-21   map ID (16), x (16), y (16), checksum (8), then 1 if party rows
--          follow. Map ID 0 means no position, like in an instance.
-- Then, only while you're in a group, a row of 30 squares each for party1-4:
--   realm (16) and player ID (32) from their GUID, Player-<realm>-<player ID>,
--   x (16), y (16), checksum (8), and 2 unused bits. All zeros when the slot
--   is empty or they aren't on your map.

local SQUARE_PX = 3
local YOUR_SQUARES = 21
local PARTY_SQUARES = 30
local PARTY = { "party1", "party2", "party3", "party4" }
local UPDATE_SECONDS = 0.1

local strip = CreateFrame("Frame", nil, WorldFrame)
strip:SetFrameStrata("TOOLTIP")
strip:SetPoint("TOPLEFT")
-- The party rows have their own frame, so they can hide while you're alone.
local partyFrame = CreateFrame("Frame", nil, strip)
partyFrame:SetPoint("TOPLEFT")

-- rows[1] is yours, then rows[2] to rows[5] are party1 to party4
local rows = { {} }
for i = 1, YOUR_SQUARES do
    rows[1][i] = strip:CreateTexture(nil, "OVERLAY")
end
for r = 2, #PARTY + 1 do
    rows[r] = {}
    for i = 1, PARTY_SQUARES do
        rows[r][i] = partyFrame:CreateTexture(nil, "OVERLAY")
    end
end
rows[1][1]:SetColorTexture(1, 0, 1)
rows[1][2]:SetColorTexture(0, 1, 0)

-- Makes each square exactly SQUARE_PX screen pixels, whatever the UI scale.
local function layout()
    local _, screenHeight = GetPhysicalScreenSize()
    local size = SQUARE_PX * 768 / screenHeight / strip:GetEffectiveScale()
    strip:SetSize(size * YOUR_SQUARES, size)
    partyFrame:SetSize(size * PARTY_SQUARES, size * #rows)
    for r, row in ipairs(rows) do
        for i, square in ipairs(row) do
            square:SetSize(size, size)
            square:SetPoint("TOPLEFT", (i - 1) * size, (1 - r) * size)
        end
    end
end

-- Adds `value to `butes` as `count` bytes, highest first.
local function addBytes(bytes, value, count)
    for i = count - 1, 0, -1 do
        bytes[#bytes + 1] = math.floor(value / 256 ^ i) % 256
    end
end

local function addCoordinates(bytes, value)
    addBytes(bytes, math.floor(math.min(math.max(value, 0), 1) * 65535 + 0.5), 2)
end

local function toBits(bytes)
    local bits, sum = {}, 0
    local function add(byte)
        for i = 7, 0, -1 do
            bits[#bits + 1] = math.floor(byte / 2 ^ i) % 2
        end
    end
    for _, byte in ipairs(bytes) do
        add(byte)
        sum = sum + byte
    end
    add(255 - sum % 256)
    return bits
end

local function yourBits(map, x, y, grouped)
    local bytes = {}
    addBytes(bytes, map, 2)
    addCoordinates(bytes, x)
    addCoordinates(bytes, y)
    local bits = toBits(bytes)
    bits[#bits + 1] = grouped and 1 or 0
    return bits
end

-- A party member on your map, or all zeros for an empty slot or anyone else.
local function partyBits(unit, map)
    local bytes = { 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 }
    local realm, id = (UnitGUID(unit) or ""):match("^Player%-(%d+)%-(%x+)$")
    realm, id = tonumber(realm), tonumber(id or "", 16)
    local pos = realm and id and realm < 2 ^ 16 and id < 2 ^ 32 and map ~= 0
        and C_Map.GetBestMapForUnit(unit) == map
        and C_Map.GetPlayerMapPosition(map, unit)
    if pos then
        local x, y = pos:GetXY()
        bytes = {}
        addBytes(bytes, realm, 2)
        addBytes(bytes, id, 4)
        addCoordinates(bytes, x)
        addCoordinates(bytes, y)
    end
    return toBits(bytes)
end

-- Colors a row's square from `first` on, 3 bits each. Bits past the end are 0.
local function paint(row, first, bits)
    for i = first, #row do
        local n = 3 * (i - first)
        row[i]:SetColorTexture(bits[n + 1] or 0, bits[n + 2] or 0, bits[n + 3] or 0)
    end
end

local painted = {}
local elapsed = UPDATE_SECONDS

local function update(_, dt)
    elapsed = elapsed + dt
    if elapsed < UPDATE_SECONDS then return end
    elapsed = 0

    local map = C_Map.GetBestMapForUnit("player")
    local pos = map and C_Map.GetPlayerMapPosition(map, "player")
    local x, y = 0, 0
    if pos then
        x, y = pos:GetXY()
    else
        map = 0
    end
    local grouped = IsInGroup()
    partyFrame:SetShown(grouped)

    local rowBits = { yourBits(map, x, y, grouped) }
    for i, unit in ipairs(PARTY) do
        rowBits[i + 1] = partyBits(unit, map)
    end
    -- Only rows that changed are painted again.
    for r, bits in ipairs(rowBits) do
        local key = table.concat(bits)
        if key ~= painted[r] then
            painted[r] = key
            paint(rows[r], r == 1 and 3 or 1, bits)
        end
    end
end

strip:RegisterEvent("PLAYER_ENTERING_WORLD")
strip:RegisterEvent("DISPLAY_SIZE_CHANGED")
strip:RegisterEvent("UI_SCALE_CHANGED")
strip:SetScript("OnEvent", layout)
strip:SetScript("OnUpdate", update)