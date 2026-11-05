-- ORBIS probe 120 (#15, unattended via scripts/dev/octane-run.sh <probe> <scene.orbx>):
-- Octane Daylight's ground blend vs Iray's Sun-Sky horizon blur.
-- Loads Documents/orbis_scene.orbx (Calib-L-Mi-SunSky-Look), logs the
-- daylight node's ground pins (info, value), then renders with
-- groundBlendAngle 1 / 5 / 11.5 / 30 (degrees) at groundStartAngle 0, then
-- start 10, under the Octane (1) and Hosek-Wilkie (3) models. A pin without
-- an input node (groundBlendAngle unless the export writes it) can't be set
-- from here: it stays 5 (logged) - export the value and compare "asis".
-- Output: ~/Documents/orbis_p120_*.png, orbis_probe120.txt, orbis_probe.done.
local home = os.getenv("HOME") or "/tmp"
local docs = home .. "/Documents/"
local lines = {}
local function log(s)
    lines[#lines + 1] = tostring(s)
    local f = io.open(docs .. "orbis_probe120.txt", "w")
    if f then f:write(table.concat(lines, "\n"), "\n"); f:close() end
end
local function show(v)
    if type(v) == "table" then local t = {} for k, x in pairs(v) do t[#t+1] = tostring(k).."="..show(x) end return "{"..table.concat(t, ",").."}" end
    return tostring(v)
end
log("start " .. os.date())
local ok, err = pcall(function()
    -- project.load as File > Open: importFromFile + copyFrom turned the
    -- daylight model 1 into 3 (probe 121).
    log("load " .. tostring(octane.project.load(docs .. "orbis_scene.orbx")))
    local scene = octane.project.getSceneGraph()
    local rt
    for _, item in ipairs(scene:getOwnedItems()) do
        local p = item:getProperties()
        if p.type == octane.NT_RENDERTARGET and p.name ~= "Preview Render Target" and not rt then rt = item end
    end
    local kernel = rt:getInputNode(octane.P_KERNEL)
    for i = 1, kernel:getPinCount() do
        if kernel:getPinInfoIx(i).name == "maxsamples" then kernel:getInputNodeIx(i):setAttribute(octane.A_VALUE, { 64, 0, 0, 0 }, true) end
    end
    -- The tone LUT doesn't load from the import (see octane-render.lua):
    -- a low exposure keeps sky and ground off the clip for measuring.
    local im = rt:getInputNode(octane.P_IMAGER)
    for i = 1, im:getPinCount() do
        local n = im:getPinInfoIx(i).name
        if n == "exposure" then local c = im:getInputNodeIx(i); if c then c:setAttribute(octane.A_VALUE, { 0.05, 0, 0, 0 }, true) else im:setPinValueIx(i, 0.05, true) end end
        if n == "lut" then local c = im:getInputNodeIx(i); log("lut " .. tostring(c)) end
    end
    local env = rt:getInputNode(octane.P_ENVIRONMENT)
    log("environment type " .. tostring(env:getProperties().type))
    local pin = {}
    for i = 1, env:getPinCount() do
        local info = env:getPinInfoIx(i)
        pin[info.name] = i
        if info.name:find("ground") or info.name == "model" then
            local c = env:getInputNodeIx(i)
            log("pin " .. info.name .. " " .. show(info) .. " value=" .. (c and c:getAttributeCount() > 0 and show(c:getAttributeIx(1)) or "nil"))
        end
    end
    -- A pin without an input node gets a float node (setPinValueIx alone
    -- may not take), read back into the log.
    local function set(name, v)
        local i = pin[name]
        local c = env:getInputNodeIx(i)
        if not c then
            local okc = pcall(function()
                local n = octane.node.create{ type = octane.NT_FLOAT, name = name, graphOwner = env:getProperties().graphOwner }
                env:connectToIx(i, n)
            end)
            c = env:getInputNodeIx(i)
            if not c then
                env:setPinValueIx(i, { v, 0, 0, 0 }, true)
                log("  " .. name .. " (no node, connect " .. tostring(okc) .. ") pin value = " .. show(env:getPinValueIx(i)))
                return
            end
        end
        c:setAttribute(octane.A_VALUE, { v, 0, 0, 0 }, true)
        local r = env:getInputNodeIx(i):getAttributeIx(1)
        log("  " .. name .. " = " .. tostring(type(r) == "table" and r[1] or r))
    end
    local function render(name)
        local t = os.time()
        octane.render.start{ renderTargetNode = rt }
        octane.render.saveImage(docs .. "orbis_p120_" .. name .. ".png", octane.imageSaveFormat.PNG_8)
        log(name .. ": " .. (os.time() - t) .. " s")
    end
    render("asis")
    for _, model in ipairs({ 1, 3 }) do
        local m = env:getInputNodeIx(pin["model"])
        if m then m:setAttribute(octane.A_VALUE, model, true) else env:setPinValueIx(pin["model"], model, true) end
        log("model now " .. show(m and m:getAttributeIx(1)))
        for _, b in ipairs({ 1, 5, 11.5, 30 }) do
            set("groundStartAngle", 0)
            set("groundBlendAngle", b)
            render("m" .. model .. "_blend" .. b)
        end
        set("groundBlendAngle", 5)
        set("groundStartAngle", 10)
        render("m" .. model .. "_start10")
    end
end)
if not ok then log("ERROR " .. tostring(err)) end
log("done " .. os.date())
local f = io.open(docs .. "orbis_probe.done", "w")
if f then f:write("done\n"); f:close() end
