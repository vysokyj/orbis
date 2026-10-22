-- Octane X Lua probe: light decomposition of an open ORBIS export.
-- Renders the scene at exposure 0.5, then each light group alone (env, spot,
-- point = UsdLux emitters, emissive = emissive materials; the others' power
-- at 0). Output in Octane X's container Documents: orbis_p25_*.png and the
-- log orbis_probe25.txt; fit with scripts/dev/octane-light-fit.py decompose.
-- Install: copy to Documents/ORBIS Scripts/ in Octane X's container, run
-- via Script > ORBIS probe.

local SAMPLES = 256
local lines = {}
local home = os.getenv("HOME") or "/tmp"
local logpath = home .. "/Documents/orbis_probe25.txt"
local function log(s)
    lines[#lines + 1] = tostring(s)
    local f = io.open(logpath, "w")
    if f then f:write(table.concat(lines, "\n"), "\n"); f:close() end
end
log("start")
local function try(label, fn)
    local ok, err = pcall(fn)
    if not ok then log("ERROR " .. label .. ": " .. tostring(err)) end
    return ok
end
local function show(v)
    if type(v) == "table" then
        local t = {}
        for _, x in ipairs(v) do t[#t + 1] = tostring(x) end
        return table.concat(t, " ")
    end
    return tostring(v)
end

local scene = octane.project.getSceneGraph()
local rt, usd
for _, item in ipairs(scene:getOwnedItems()) do
    local p = item:getProperties()
    if p.type == octane.NT_RENDERTARGET and p.name ~= "Preview Render Target" and not rt then rt = item end
    if p.name and p.name:find("%(DAZ 3D%)") then usd = item end
end
log("rt " .. tostring(rt and rt:getProperties().name) .. " usd " .. tostring(usd and usd:getProperties().name))
if not rt or not usd then
    octane.gui.showDialog{ type = octane.gui.dialogType.BUTTON_DIALOG, title = "ORBIS probe 25",
        text = "No ORBIS export open: open the .orbx (File > Open) first." }
    return
end

local function pins(node)
    local t = {}
    for i = 1, node:getPinCount() do t[node:getPinInfoIx(i).name] = i end
    return t
end

-- Material nodes of the USD graph by name.
local mats = {}
for _, item in ipairs(usd:getOwnedItems()) do
    local ok, m = pcall(function() return item:getInputNodeIx(1) end)
    if ok and m then mats[#mats + 1] = { name = item:getProperties().name, node = m } end
end
log("materials " .. #mats)

local function dump(entry)
    local m = entry.node
    log("== " .. entry.name .. " type " .. tostring(m:getProperties().type))
    for i = 1, m:getPinCount() do
        local name = m:getPinInfoIx(i).name
        local n = m:getInputNodeIx(i)
        local v = "-"
        if n then
            local p = n:getProperties()
            v = "node type " .. tostring(p.type)
            pcall(function() v = v .. " value " .. show(n:getAttribute(octane.A_VALUE)) end)
            pcall(function() v = v .. " file " .. show(n:getAttribute(octane.A_FILENAME)) end)
        end
        log("  " .. i .. " " .. name .. " = " .. v)
    end
end
for _, e in ipairs(mats) do
    if e.name:find("Torso") or e.name:find("Swimsuit_Base") then try("dump " .. e.name, function() dump(e) end) end
end

local kernel = rt:getInputNode(octane.P_KERNEL)
local samplesNode = kernel:getInputNodeIx(pins(kernel)["maxsamples"])
local origSamples = samplesNode:getAttribute(octane.A_VALUE)
samplesNode:setAttribute(octane.A_VALUE, { SAMPLES, 0, 0, 0 }, true)

local function save(name)
    local path = home .. "/Documents/orbis_p25_" .. name .. ".png"
    local ok, res = pcall(octane.render.saveImage, path, octane.imageSaveFormat.PNG_8)
    log("saved " .. name .. ": " .. tostring(ok) .. " " .. tostring(res))
end
local function render(name)
    octane.render.start{ renderTargetNode = rt }
    save(name)
end

-- Set a pin's owned value node on every material that has the pin and a
-- plain value node there (textures are left alone unless `replace`).
local undo = {}
local function setAll(pin, value)
    local count = 0
    for _, e in ipairs(mats) do
        local ix = pins(e.node)[pin]
        if ix then
            local n = e.node:getInputNodeIx(ix)
            if n then
                local ok, old = pcall(function() return n:getAttribute(octane.A_VALUE) end)
                if ok and old ~= nil then
                    undo[#undo + 1] = { n, old }
                    n:setAttribute(octane.A_VALUE, value, true)
                    count = count + 1
                end
            end
        end
    end
    log("set " .. pin .. " = " .. show(value) .. " on " .. count)
end
local function restore()
    for k = #undo, 1, -1 do undo[k][1]:setAttribute(octane.A_VALUE, undo[k][2], true) end
    undo = {}
end

local imager = rt:getInputNodeIx(pins(rt)["imager"])
local function ipin(name) return imager:getInputNodeIx(pins(imager)[name]) end
local orig = {}
for _, n in ipairs({ "exposure", "highlightCompression", "acesToneMapping", "white_balance", "saturation", "response", "gamma" }) do
    orig[n] = ipin(n):getAttribute(octane.A_VALUE)
    log("imager " .. n .. " = " .. show(orig[n]))
end
local function wait(seconds)
    local t = os.clock() + seconds
    while os.clock() < t do end
end

-- Light decomposition: render each light group alone (others' power 0) at
-- exposure 0.5, to fit DAZ's render as a weighted sum of them.
local groups = { env = {}, spot = {}, point = {}, emissive = {} }
local function addPower(group, emitter, label)
    local ix = pins(emitter)["power"]
    if not ix then log("no power pin on " .. label); return end
    local n = emitter:getInputNodeIx(ix)
    if not n then log("no power node on " .. label); return end
    local ok, v = pcall(function() return n:getAttribute(octane.A_VALUE) end)
    if ok and v then
        table.insert(groups[group], { n, v })
        log(group .. ": " .. label .. " power " .. show(v))
    end
end
-- Material emitters: UsdLux lights (/Scene/Lights/) and emissive surfaces.
for _, e in ipairs(mats) do
    pcall(function()
        local inner = e.node
        local eix = pins(inner)["emission"]
        local em = eix and inner:getInputNodeIx(eix)
        if em then addPower(e.name:find("/Scene/Lights/") and "point" or "emissive", em, e.name) end
    end)
end
-- Analytic spots in the Scene graph.
for _, item in ipairs(scene:getOwnedItems()) do
    if item:getProperties().type == 402 then
        pcall(function() addPower("spot", item:getInputNodeIx(pins(item)["emission"]), item:getProperties().name) end)
    end
end
-- The environment.
pcall(function()
    local env = rt:getInputNodeIx(pins(rt)["environment"])
    log("environment type " .. tostring(env:getProperties().type))
    addPower("env", env, "environment")
end)
for g, list in pairs(groups) do log("group " .. g .. ": " .. #list) end

local function setGroup(g, on)
    for _, pv in ipairs(groups[g]) do
        local v = pv[2]
        if not on then
            v = {}
            for i = 1, #pv[2] do v[i] = 0 end
        end
        pv[1]:setAttribute(octane.A_VALUE, v, true)
    end
end
try("exposure 0.5", function() ipin("exposure"):setAttribute(octane.A_VALUE, { 0.5, 0, 0 }, true) end)
try("all", function() render("all_e05") end)
for _, only in ipairs({ "env", "spot", "point", "emissive" }) do
    if #groups[only] > 0 then
        for g in pairs(groups) do setGroup(g, g == only) end
        try(only, function() render("only_" .. only) end)
    end
end
for g in pairs(groups) do setGroup(g, true) end
for n, v in pairs(orig) do pcall(function() ipin(n):setAttribute(octane.A_VALUE, v, true) end) end

try("restore", function() samplesNode:setAttribute(octane.A_VALUE, origSamples, true) end)
log("done")
local summary = "ORBIS probe 25 finished: " .. #lines .. " lines"
print(summary)
pcall(function()
    octane.gui.showDialog{ type = octane.gui.dialogType.BUTTON_DIALOG, title = "ORBIS probe 25", text = summary }
end)
