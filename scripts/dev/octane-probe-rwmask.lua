-- ORBIS probe 67 (unattended: scripts/dev/octane-run.sh <this> <export.orbx>):
-- the random walk's dark floor on a real (Enhanced) skin export (#19): the
-- export as is (subsurface = the detail mask), subsurface 1 everywhere, and
-- subsurface 0 (a plain diffuse of the same baked colour). Plain imager.
-- Output: ~/Documents/orbis_p67_{mask,one,zero}.png, orbis_probe67.txt.
local SAMPLES = 256
local home = os.getenv("HOME") or "/tmp"
local docs = home .. "/Documents/"
local lines = {}
local function log(s)
    lines[#lines + 1] = tostring(s)
    local f = io.open(docs .. "orbis_probe67.txt", "w")
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
local skin = {}
try("materials", function()
    for _, item in ipairs(usd:getOwnedItems()) do
        local ok, m = pcall(function() return item:getInputNodeIx(1) end)
        if ok and m and m:getProperties().type == 505 and pins(m)["subsurface"] then
            skin[#skin + 1] = m
            log("skin " .. tostring(item:getProperties().name))
        end
    end
    log(#skin .. " skin materials")
end)
local function setNode(m, pin, typ, value)
    local ix = pins(m)[pin]
    if not ix then return end
    local c = m:getInputNodeIx(ix)
    if not (c and c:getProperties().type == typ) then
        c = octane.node.create{ type = typ, name = pin, pinOwnerNode = m, pinOwnerIx = ix }
    end
    c:setAttribute(octane.A_VALUE, value, true)
end
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
    local lut = p["lut"] and im:getInputNodeIx(p["lut"])
    if lut then pcall(function() im:disconnectIx(p["lut"]) end) end
    log("lut left " .. tostring(p["lut"] and im:getInputNodeIx(p["lut"]) ~= nil))
    for _, n in ipairs({ "exposure", "response", "gamma", "acesToneMapping", "white_balance", "saturation" }) do
        if p[n] then pcall(function() log("imager " .. n .. " " .. tostring(im:getInputNodeIx(p[n]):getAttribute(octane.A_VALUE)[1] or im:getInputNodeIx(p[n]):getAttribute(octane.A_VALUE))) end) end
    end
end)
local function render(name)
    local t = os.time()
    octane.render.start{ renderTargetNode = rt }
    log("rendered " .. name .. " in " .. (os.time() - t) .. " s: " .. tostring(octane.render.saveImage(docs .. "orbis_p67_" .. name .. ".png", octane.imageSaveFormat.PNG_8)))
end
local function paint(g, sss)
    for _, m in ipairs(skin) do
        setNode(m, "baseColor", octane.NT_TEX_RGB, { g, g, g })
        setNode(m, "subsurfaceColor", octane.NT_TEX_RGB, { g, g, g })
        setNode(m, "subsurface", octane.NT_TEX_FLOAT, sss)
    end
end
local function subsurface(v)
    for _, m in ipairs(skin) do setNode(m, "subsurface", octane.NT_TEX_FLOAT, v) end
end
try("mask", function() render("mask") end)
try("one", function() subsurface(1); render("one") end)
try("zero", function() subsurface(0); render("zero") end)
pcall(function() octane.project.reset() end)
log("done")
local f = io.open(docs .. "orbis_probe.done", "w"); if f then f:write(os.date(), "\n"); f:close() end
