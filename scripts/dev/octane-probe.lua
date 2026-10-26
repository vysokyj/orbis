-- ORBIS probe (56th run, unattended via scripts/dev/octane-run.sh):
-- Gladiator.orbx — the Gladio outfit (PBRSkin: dual lobe at reflectivity
-- 0.25, top coat 0.25, metallic maps) renders its red cloth pink and its
-- armour too bright. Plain imager (Gamma 2.2 response, exposure 1.4, no
-- LUT), to compare with Iray's render at the calibrated tone mapping.
-- Variants on every outfit material (Universal, name with "__"):
-- as exported; specular 0.5; specular 0.5 and no coating; specular 0 and no
-- coating.
-- Output: ~/Documents/orbis_p95_*.png, orbis_probe95.txt, orbis_probe.done.

local SAMPLES = 128
local home = os.getenv("HOME") or "/tmp"
local docs = home .. "/Documents/"
local lines = {}
local function log(s)
    lines[#lines + 1] = tostring(s)
    local f = io.open(docs .. "orbis_probe95.txt", "w")
    if f then f:write(table.concat(lines, "\n"), "\n"); f:close() end
end
local function try(label, fn)
    local ok, err = pcall(fn)
    if not ok then log("ERROR " .. label .. ": " .. tostring(err)) end
    return ok
end
local function pins(node)
    local t = {}
    for i = 1, node:getPinCount() do t[node:getPinInfoIx(i).name] = i end
    return t
end

log("start " .. os.date())
local rt, usd
try("load", function()
    log("load: " .. tostring(octane.project.load(docs .. "orbis_scene.orbx")))
    for _, item in ipairs(octane.project.getSceneGraph():getOwnedItems()) do
        local p = item:getProperties()
        if p.type == octane.NT_RENDERTARGET and p.name ~= "Preview Render Target" and not rt then rt = item end
        if p.name and p.name:find("%(DAZ 3D%)") then usd = item end
    end
end)

-- The outfit's Universal materials.
local outfit = {}
try("materials", function()
    for _, item in ipairs(usd:getOwnedItems()) do
        local name = item:getProperties().name or ""
        local ok, m = pcall(function() return item:getInputNodeIx(1) end)
        if ok and m and name:find("__") and m:getProperties().type == 130 then
            outfit[#outfit + 1] = { name = name, node = m }
        end
    end
    log(#outfit .. " outfit materials")
end)

-- A float on a material pin as an owned float texture.
local function setPin(m, pin, value)
    local ix = pins(m)[pin]
    if not ix then return end
    local c = m:getInputNodeIx(ix)
    if c and c:getProperties().type == octane.NT_TEX_FLOAT then
        c:setAttribute(octane.A_VALUE, value, true)
    else
        local n = octane.node.create{ type = octane.NT_TEX_FLOAT, name = pin, pinOwnerNode = m, pinOwnerIx = ix }
        n:setAttribute(octane.A_VALUE, value, true)
    end
end
local function forOutfit(fn)
    for _, e in ipairs(outfit) do try(e.name, function() fn(e.node) end) end
end

-- Plain imager: no tone LUT.
try("imager", function()
    local im = rt:getInputNode(octane.P_IMAGER)
    local p = pins(im)
    local function set(pin, v)
        local c = im:getInputNodeIx(p[pin])
        local cur = c:getAttribute(octane.A_VALUE)
        if type(cur) == "table" then cur[1] = v; c:setAttribute(octane.A_VALUE, cur, true) else c:setAttribute(octane.A_VALUE, v, true) end
    end
    local kernel = rt:getInputNode(octane.P_KERNEL)
    kernel:getInputNodeIx(pins(kernel)["maxsamples"]):setAttribute(octane.A_VALUE, { SAMPLES, 0, 0, 0 }, true)
    set("response", 403); set("gamma", 1); set("highlightCompression", 0)
    -- The tone LUT out: disconnected if the API allows, else a LUT that
    -- undoes its 1/16 exposure (the plain Gamma 2.2 response).
    local lut = im:getInputNodeIx(p["lut"])
    local gone = lut and pcall(function() im:disconnectIx(p["lut"]) end) and not im:getInputNodeIx(p["lut"])
    if gone or not lut then
        set("exposure", 1.4)
        log("LUT disconnected")
    else
        lut:setAttribute(octane.A_PACKAGE, "", true)
        lut:setAttribute(octane.A_FILENAME, docs .. "orbis_plain.cube", true)
        log("plain LUT")
    end
end)
local function render(name)
    local t = os.time()
    octane.render.start{ renderTargetNode = rt }
    log("rendered " .. name .. " in " .. (os.time() - t) .. " s: " .. tostring(octane.render.saveImage(docs .. "orbis_p95_" .. name .. ".png", octane.imageSaveFormat.PNG_8)))
end

try("base", function() render("base") end)
try("spec05", function() forOutfit(function(m) setPin(m, "specular", 0.5) end); render("spec05") end)
try("spec05_nocoat", function() forOutfit(function(m) setPin(m, "coating", 0) end); render("spec05_nocoat") end)
try("spec0_nocoat", function() forOutfit(function(m) setPin(m, "specular", 0) end); render("spec0_nocoat") end)

-- Drop the changes, so quitting doesn't ask to save them.
pcall(function() octane.project.reset() end)
log("done")
local f = io.open(docs .. "orbis_probe.done", "w"); if f then f:write(os.date(), "\n"); f:close() end
