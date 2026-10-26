-- ORBIS probe (54th run): Gladiator.orbx (#14). Iray's tone curve with Burn
-- 0, Crush 0.07, Gamma 1.857 fits (c / (1 + c))^(1 + crush) then ^(1/gamma)
-- within 1.7/255. Can the imager apply it as a custom LUT? orbis_tone_1d.cube
-- (4096 entries) and orbis_tone_3d.cube (33^3) map the Gamma 2.2 response of
-- c / 16 (exposure 1.4 / 16) to Iray's output. Logs the LUT node types and the
-- imager's `order`; renders a linear EXR (exposure 0.25) and both LUTs.
-- Output: ~/Documents/orbis_p93_*.png|exr, orbis_probe93.txt.
-- Close the scene without saving afterwards.

local SAMPLES = 64
local lines = {}
local home = os.getenv("HOME") or "/tmp"
local logpath = home .. "/Documents/orbis_probe93.txt"
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
        for _, x in ipairs(v) do t[#t + 1] = show(x) end
        return "{" .. table.concat(t, " ") .. "}"
    end
    return tostring(v)
end

local scene = octane.project.getSceneGraph()
local rt
for _, item in ipairs(scene:getOwnedItems()) do
    local p = item:getProperties()
    if p.type == octane.NT_RENDERTARGET and p.name ~= "Preview Render Target" and not rt then rt = item end
end
log("rt " .. tostring(rt and rt:getProperties().name))
if not rt then
    octane.gui.showDialog{ type = octane.gui.dialogType.BUTTON_DIALOG, title = "ORBIS probe 93",
        text = "No ORBIS export open: open Gladiator.orbx (File > Open) first." }
    return
end

for _, k in ipairs({ "NT_GEO_VOLUME", "NT_MED_VOLUME", "NT_MED_STANDARD_VOLUME", "NT_MED_SCATTERING", "NT_GEO_GROUP",
    "NT_GEO_PLACEMENT", "NT_TRANSFORM_VALUE", "NT_TRANSFORM_3D", "NT_FLOAT", "NT_TEX_RGB", "NT_TEX_FLOAT" }) do
    log("enum " .. k .. " = " .. tostring(octane[k]))
end

local function pins(node)
    local t = {}
    for i = 1, node:getPinCount() do t[node:getPinInfoIx(i).name] = i end
    return t
end
local function describe(node, label)
    log("== " .. label .. " type " .. tostring(node:getProperties().type))
    for i = 1, node:getPinCount() do
        local info = node:getPinInfoIx(i)
        local c = node:getInputNodeIx(i)
        local v = c and ("node type " .. tostring(c:getProperties().type)) or "-"
        if c then pcall(function() v = v .. " value " .. show(c:getAttribute(octane.A_VALUE)) end) end
        log("  pin " .. i .. " " .. tostring(info.name) .. " (pin type " .. tostring(info.type) .. ") = " .. v)
    end
    try("attrs " .. label, function()
        for i = 1, node:getAttributeCount() do
            local info = node:getAttributeInfoIx(i)
            if i == 1 then
                local keys = {}
                for k, x in pairs(info) do keys[#keys + 1] = k .. "=" .. show(x) end
                log("  attr info keys: " .. table.concat(keys, ", "))
            end
            local an = info.name
            if not an and info.id then
                pcall(function() an = octane.apiinfo.attributeName(info.id) end)
                if not an then pcall(function() an = octane.apiinfo.getAttributeName(info.id) end) end
            end
            info.name = an or ("id " .. tostring(info.id))
            local v = "?"
            pcall(function() v = show(node:getAttributeIx(i)) end)
            if #v > 300 then v = v:sub(1, 300) .. "…" end
            log("  attr " .. i .. " " .. tostring(info.name) .. " (attr type " .. tostring(info.type) .. ") = " .. v)
        end
    end)
end
-- A float or colour on a pin by name, as an owned value node.
local function setPin(node, pin, value)
    local ix = pins(node)[pin]
    if not ix then log("no pin " .. pin); return end
    -- Change the value node in place when there is one (its type fits the pin).
    local c = node:getInputNodeIx(ix)
    local v
    if type(value) == "boolean" then
        v = (c and c:getProperties().type == octane.NT_BOOL) and c or octane.node.create{ type = octane.NT_BOOL, name = pin, pinOwnerNode = node, pinOwnerIx = ix }
        v:setAttribute(octane.A_VALUE, value, true)
    elseif type(value) == "table" then
        v = (c and c:getProperties().type == octane.NT_TEX_RGB) and c or octane.node.create{ type = octane.NT_TEX_RGB, name = pin, pinOwnerNode = node, pinOwnerIx = ix }
        v:setAttribute(octane.A_VALUE, value, true)
    else
        local ok = c and pcall(function() c:setAttribute(octane.A_VALUE, { value, 0, 0, 0 }, true) end)
        if not ok then
            v = octane.node.create{ type = octane.NT_FLOAT, name = pin, pinOwnerNode = node, pinOwnerIx = ix }
            v:setAttribute(octane.A_VALUE, { value, 0, 0, 0 }, true)
        end
    end
    log("set " .. pin .. " = " .. show(value))
end

local kernel = rt:getInputNode(octane.P_KERNEL)
local samplesNode = kernel:getInputNodeIx(pins(kernel)["maxsamples"])
local origSamples = samplesNode:getAttribute(octane.A_VALUE)
samplesNode:setAttribute(octane.A_VALUE, { SAMPLES, 0, 0, 0 }, true)

local fmt = octane.imageSaveFormat
local function save(name)
    local keys = {}
    for k2, _ in pairs(fmt) do keys[#keys + 1] = k2 end
    if name == "linear" then log("formats: " .. table.concat(keys, " ")) end
    local f, ext = fmt.EXR_32 or fmt.EXR or fmt.EXR_16 or fmt.PNG_16, ".exr"
    if f == fmt.PNG_16 then ext = ".png" end
    local path = home .. "/Documents/orbis_p93_" .. name .. ext
    local ok, res = pcall(octane.render.saveImage, path, f)
    log("saved " .. name .. ": " .. tostring(ok) .. " " .. tostring(res))
    pcall(octane.render.saveImage, home .. "/Documents/orbis_p93_" .. name .. ".png", fmt.PNG_8)
end
local function render(name)
    local t = os.time()
    octane.render.start{ renderTargetNode = rt }
    log("rendered " .. name .. " in " .. (os.time() - t) .. " s")
    save(name)
end

local imager
try("imager", function()
    imager = rt:getInputNode(octane.P_IMAGER)
    describe(imager, "imager")
end)
-- An imager pin's value node, set in place (ints for enums).
local function setImager(pin, value)
    local c = imager:getInputNodeIx(pins(imager)[pin])
    local cur = c:getAttribute(octane.A_VALUE)
    if type(cur) == "table" then
        cur[1] = value
        c:setAttribute(octane.A_VALUE, cur, true)
    else
        c:setAttribute(octane.A_VALUE, value, true)
    end
    log("imager " .. pin .. " = " .. show(value))
end
try("lut types", function()
    for k, v in pairs(octane) do
        if type(k) == "string" and k:find("^NT_") and k:find("LUT") then log("enum " .. k .. " = " .. tostring(v)) end
    end
    local info = imager:getPinInfoIx(pins(imager)["order"])
    local keys = {}
    for k, v in pairs(info) do keys[#keys + 1] = k .. "=" .. show(v) end
    log("order pin info: " .. table.concat(keys, ", "))
    local linfo = imager:getPinInfoIx(pins(imager)["lut"])
    keys = {}
    for k, v in pairs(linfo) do keys[#keys + 1] = k .. "=" .. show(v) end
    log("lut pin info: " .. table.concat(keys, ", "))
end)
try("linear", function()
    setImager("response", 400)
    setImager("gamma", 1)
    setImager("highlightCompression", 0)
    setImager("exposure", 0.25)
    render("linear")
end)
local lutNode
local function useLut(file)
    if not lutNode then
        local t = octane.NT_LUT_CUSTOM or octane.NT_OCIO_LUT or octane.NT_LUT
        lutNode = octane.node.create{ type = t, name = "Iray tone", pinOwnerNode = imager, pinOwnerIx = pins(imager)["lut"] }
        describe(lutNode, "lut")
    end
    lutNode:setAttribute(octane.A_FILENAME, home .. "/Documents/" .. file, true)
    pcall(function() lutNode:evaluate() end)
    describe(lutNode, "lut " .. file)
end
try("lut setup", function()
    setImager("response", 403)
    setImager("gamma", 1)
    setImager("highlightCompression", 0)
    setImager("exposure", 1.4 / 16)
end)
try("lut1d", function() useLut("orbis_tone_1d.cube"); render("lut1d") end)
try("lut3d", function() useLut("orbis_tone_3d.cube"); render("lut3d") end)

try("restore", function() samplesNode:setAttribute(octane.A_VALUE, origSamples, true) end)
log("done")
local summary = "ORBIS probe 93 finished: " .. #lines .. " lines. Close the scene without saving."
print(summary)
pcall(function()
    octane.gui.showDialog{ type = octane.gui.dialogType.BUTTON_DIALOG, title = "ORBIS probe 93", text = summary }
end)
