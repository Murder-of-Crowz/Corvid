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
-- Addons can't write files while the game runs, so Corvid paints your position
-- into a row of small squares in the top left corner, and the desktop app reads
-- them off the screen. Each square holds 3 bits, one per color channel, and a
-- channel is only ever full on or off, so gamma and brightness can't change it.
--
-- Squares, left to right:
-- 1, 2     magenta then green, so the app can tell the strip is there
-- 3-21     56 bits, highest first: map ID (16), x (16), y (16), checksum (8),
--          and one unused bit.
-- x and y are 0-1 scaled to 0-65535. Map ID 0 means no position, like in an
-- instance. The checksum is 255 minus the sum of the six data bytes, mod 256.

local SQUARE_PX = 3
local DATA_SQUARES = 19
local UPDATE_SECONDS = 0.1

local strip = CreateFrame("Frame", nil, WorldFrame)
strip:SetFrameStrata("TOOLTIP")
strip:SetPoint("TOPLEFT")

local squares = {}
for i = 1, 2 + DATA_SQUARES do
    squares[i] = strip:CreateTexture(nil, "OVERLAY")
end
squares[1]:SetColorTexture(1, 0, 1)
squares[2]:SetColorTexture(0, 1, 0)

-- Makes each square exactly SQUARE_PX screen pixels, whatever the UI scale.
local function layout()
    local _, screenHeight = GetPhysicalScreenSize()
    local size = SQUARE_PX * 768 / screenHeight / strip:GetEffectiveScale()
    strip:SetSize(size * #squares, size)
    for i, square in ipairs(squares) do
        square:SetSize(size, size)
        square:SetPoint("TOPLEFT", (i-1) * size, 0)
    end
end

local function addBits(bits, value, count)
    for i = count - 1, 0, -1 do
        bits[#bits + 1] = math.floor(value / 2 ^ i) % 2
    end
end

local function toBits(map, x, y)
    local x16 = math.floor(math.min(math.max(x, 0), 1) * 65535 + 0.5)
    local y16 = math.floor(math.min(math.max(y, 0), 1) * 65535 + 0.5)
    local bytes = {
        math.floor(map / 256) % 256, map % 256,
        math.floor(x16 / 256), x16 % 256,
        math.floor(y16 / 256), y16 % 256,
    }
    local bits, sum = {}, 0
    for _, byte in ipairs(bytes) do
        addBits(bits, byte, 8)
        sum = sum + byte
    end
    addBits(bits, 255 - sum % 256, 8)
    return bits
end

local lastMap, lastX, lastY
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
    if map == lastMap and x == lastX and y == lastY then return end
    lastMap, lastX, lastY = map, x, y

    local bits = toBits(map, x, y)
    for i = 1, DATA_SQUARES do
        local r, g, b = bits[3 * i - 2], bits[3 * i - 1], bits[3 * i] or 0
        squares[2 + i]:SetColorTexture(r, g, b)
    end
end

strip:RegisterEvent("PLAYER_ENTERING_WORLD")
strip:RegisterEvent("DISPLAY_SIZE_CHANGED")
strip:RegisterEvent("UI_SCALE_CHANGED")
strip:SetScript("OnEvent", layout)
strip:SetScript("OnUpdate", update)