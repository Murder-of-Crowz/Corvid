local f = CreateFrame("Frame")
f:RegisterEvent("PLAYER_ENTERING_WORLD")
f:SetScript("OnEvent", function()
    pcall(SetCVar, "advancedCombatLogging", 1)
    LoggingCombat(true)
    LoggingChat(true)
    print("WFTest: combat", LoggingCombat(), "chat", LoggingChat(),
          "advanced", GetCVar("advancedCombatLogging"))
end)

SLASH_WFTEST1 = "/corvid"
SlashCmdList.WFTEST = function()
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