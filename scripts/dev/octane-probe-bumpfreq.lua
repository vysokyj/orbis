-- ORBIS probe (#85, unattended via scripts/dev/octane-run.sh <this> <scene.orbx>):
-- how Octane's bump turns a height map into a shading normal, per spatial
-- frequency. The Info channel kernel renders, at one sample a pixel without
-- pixel filtering (a point sample each), the shading normal (type 1)
-- without and with "Bump and normal mapping", and the texture coordinates
-- (twice: the samples must land where they did). scripts/dev/bump-freq.py
-- predicts each pixel's slope from the height map at its UV under each
-- derivative rule and regresses the measured tilt on it.
-- Output: ~/Documents/orbis_bf_{flat,bump,uv,uv2}.png (16-bit),
-- orbis_probe_bumpfreq.txt, orbis_probe.done.

local SAMPLES = 1
local home = os.getenv("HOME") or "/tmp"
local docs = home .. "/Documents/"
local lines = {}
local function log(s)
    lines[#lines + 1] = tostring(s)
    local f = io.open(docs .. "orbis_probe_bumpfreq.txt", "w")
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
try("enums", function()
    local ks = {}
    for k, v in pairs(octane) do
        if type(k) == "string" and (k:find("^IC_") or k:find("INFO")) then ks[#ks + 1] = k .. "=" .. tostring(v) end
    end
    for k, v in pairs(octane.infoChannelType or {}) do ks[#ks + 1] = "infoChannelType." .. tostring(k) .. "=" .. tostring(v) end
    table.sort(ks)
    log("enums: " .. table.concat(ks, " "))
end)
local rt
try("load", function()
    log("load: " .. tostring(octane.project.load(docs .. "orbis_scene.orbx")))
    for _, item in ipairs(octane.project.getSceneGraph():getOwnedItems()) do
        local p = item:getProperties()
        if p.type == octane.NT_RENDERTARGET and p.name ~= "Preview Render Target" and not rt then rt = item end
    end
end)

local kernel
try("kernel", function()
    local rp = pins(rt)
    kernel = octane.node.create{ type = octane.NT_KERN_INFO, name = "info", pinOwnerNode = rt, pinOwnerIx = rp["kernel"] }
    local kp = pins(kernel)
    kernel:getInputNodeIx(kp["maxsamples"]):setAttribute(octane.A_VALUE, { SAMPLES, 0, 0, 0 }, true)
    kernel:getInputNodeIx(kp["samplingMode"]):setAttribute(octane.A_VALUE, 2, true)
end)

local function render(name, typ, bump)
    try(name, function()
        local kp = pins(kernel)
        kernel:getInputNodeIx(kp["type"]):setAttribute(octane.A_VALUE, typ, true)
        kernel:getInputNodeIx(kp["bump"]):setAttribute(octane.A_VALUE, bump, true)
        local t = os.time()
        octane.render.start{ renderTargetNode = rt }
        local fmt = octane.imageSaveFormat.PNG_16 or octane.imageSaveFormat.PNG_8
        log("rendered " .. name .. " in " .. (os.time() - t) .. " s: " .. tostring(octane.render.saveImage(docs .. "orbis_bf_" .. name .. ".png", fmt)))
    end)
end
render("flat", 1, false)
render("bump", 1, true)
render("uv", 5, true)
render("uv2", 5, true)
render("tn", 15, true) -- tangent-space normal (not used by bump-freq.py)

pcall(function() octane.project.reset() end)
log("done")
local f = io.open(docs .. "orbis_probe.done", "w"); if f then f:write(os.date(), "\n"); f:close() end
