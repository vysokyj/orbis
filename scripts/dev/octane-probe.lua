-- ORBIS probe (32nd run): Octane4.orbx — geometry shells are now refined
-- whole and cut to their visible surfaces afterwards: cut first, the NGV8
-- Color Layer's border sank up to 0.3 mm into the skin in creases and
-- random-walk skin lit it white (probes 62-70). Renders as exported.
-- Output: ~/Documents/orbis_p71_*.png, orbis_probe71.txt.

local SAMPLES = 128
local lines = {}
local home = os.getenv("HOME") or "/tmp"
local logpath = home .. "/Documents/orbis_probe71.txt"
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
    octane.gui.showDialog{ type = octane.gui.dialogType.BUTTON_DIALOG, title = "ORBIS probe 71",
        text = "No ORBIS export open: open Octane4.orbx (File > Open) first." }
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

local function dumpNode(n, depth)
    local m = n
    for i = 1, m:getPinCount() do
        local name = m:getPinInfoIx(i).name
        local c = m:getInputNodeIx(i)
        if c then
            local p = c:getProperties()
            local v = "node type " .. tostring(p.type)
            pcall(function() v = v .. " value " .. show(c:getAttribute(octane.A_VALUE)) end)
            pcall(function() v = v .. " file " .. show(c:getAttribute(octane.A_FILENAME)) end)
            log(string.rep("  ", depth) .. i .. " " .. name .. " = " .. v)
            -- Operators (multiply) and image textures: one level down.
            if depth < 3 and (p.type == 39 or name == "power") then dumpNode(c, depth + 1) end
        end
    end
end
local function matching(pattern)
    local out = {}
    for _, e in ipairs(mats) do
        if e.name:find(pattern) then out[#out + 1] = e end
    end
    return out
end
for _, pat in ipairs({ "Pubic_Hair", "Color_Layer", "Fleur_HD_new_gens" }) do
    for _, e in ipairs(matching(pat)) do
        try("dump " .. e.name, function()
            log("== " .. e.name .. " type " .. tostring(e.node:getProperties().type))
            dumpNode(e.node, 1)
        end)
    end
end

local kernel = rt:getInputNode(octane.P_KERNEL)
local samplesNode = kernel:getInputNodeIx(pins(kernel)["maxsamples"])
local origSamples = samplesNode:getAttribute(octane.A_VALUE)
samplesNode:setAttribute(octane.A_VALUE, { SAMPLES, 0, 0, 0 }, true)

local function save(name)
    -- The denoised pass when the imager's denoiser is on (saveImage writes
    -- the raw beauty, which keeps the noise).
    local path = home .. "/Documents/orbis_p71_" .. name .. ".png"
    local ok, res = pcall(octane.render.saveRenderPass, octane.renderPassId.BEAUTY_DENOISER_OUTPUT, path, octane.imageSaveFormat.PNG_8)
    if not ok or not res then ok, res = pcall(octane.render.saveImage, path, octane.imageSaveFormat.PNG_8) end
    log("saved " .. name .. ": " .. tostring(ok) .. " " .. tostring(res))
end
local function render(name)
    octane.render.start{ renderTargetNode = rt }
    save(name)
end

-- Put a constant on a material pin: change its value node in place, or
-- replace whatever drives it (a texture) with a new owned value node.
local function setPin(e, pin, nodeType, value)
    local ix = pins(e.node)[pin]
    if not ix then log("no pin " .. pin .. " on " .. e.name); return end
    local n = e.node:getInputNodeIx(ix)
    if n and n:getProperties().type == nodeType then
        n:setAttribute(octane.A_VALUE, value, true)
    else
        local ok = pcall(function()
            local v = octane.node.create{ type = nodeType, name = pin, pinOwnerNode = e.node, pinOwnerIx = ix }
            v:setAttribute(octane.A_VALUE, value, true)
        end)
        if not ok then
            local v = octane.node.create{ type = nodeType, name = pin, graphOwner = usd }
            v:setAttribute(octane.A_VALUE, value, true)
            e.node:connectToIx(ix, v)
        end
    end
    log("set " .. e.name .. " " .. pin .. " = " .. show(value))
end

try("render base", function() render("base") end)

try("restore", function() samplesNode:setAttribute(octane.A_VALUE, origSamples, true) end)
log("done")
local summary = "ORBIS probe 71 finished: " .. #lines .. " lines. Close the scene without saving."
print(summary)
pcall(function()
    octane.gui.showDialog{ type = octane.gui.dialogType.BUTTON_DIALOG, title = "ORBIS probe 71", text = summary }
end)
