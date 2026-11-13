-- ORBIS probe (#140, unattended via scripts/dev/octane-run.sh <this> <scene.orbx>):
-- what an OSL texture (117) with input pins becomes when Octane loads it
-- from a project.ocs. Walks every node of the loaded scene (the USD graph's
-- materials too) and logs each OSL texture's pins (name, type, the node
-- connected), its attributes (`result`: compiled) and the first line of
-- its shader code.
-- Output: ~/Documents/orbis_probe_oslpins.txt, orbis_probe.done.

local home = os.getenv("HOME") or "/tmp"
local docs = home .. "/Documents/"
local lines = {}
local function log(s)
    lines[#lines + 1] = tostring(s)
    local f = io.open(docs .. "orbis_probe_oslpins.txt", "w")
    if f then f:write(table.concat(lines, "\n"), "\n"); f:close() end
end
local function try(label, fn)
    local ok, err = pcall(fn)
    if not ok then log("ERROR " .. label .. ": " .. tostring(err)) end
    return ok
end
local function show(v)
    if type(v) == "table" then
        local t = {}
        for k, x in pairs(v) do t[#t + 1] = tostring(k) .. "=" .. show(x) end
        return "{" .. table.concat(t, ", ") .. "}"
    end
    return tostring(v)
end

log("start " .. os.date())
log("NT_TEX_OSL = " .. tostring(octane.NT_TEX_OSL))
try("load", function() log("load: " .. tostring(octane.project.load(docs .. "orbis_scene.orbx"))) end)

local seen = {}
local function describe(node, path)
    local p = node:getProperties()
    log("== OSL node '" .. tostring(p.name) .. "' at " .. path .. ", pins " .. node:getPinCount())
    for i = 1, node:getPinCount() do
        local info = node:getPinInfoIx(i)
        local c = node:getInputNodeIx(i)
        local cp = c and c:getProperties()
        log(string.format("  pin %d %s type %s -> %s", i, tostring(info.name), tostring(info.type), cp and (tostring(cp.type) .. " " .. tostring(cp.name)) or "nil"))
    end
    local okn, n = pcall(function() return node:getAttributeCount() end)
    for j = 1, (okn and n or 0) do
        local ai = node:getAttributeInfoIx(j)
        local v = node:getAttributeIx(j)
        if type(v) == "string" then v = v:sub(1, 80):gsub("\n", " | ") end
        log("  attr " .. tostring(ai.name) .. " = " .. show(v))
    end
end
local function walk(node, path, depth)
    if not node or depth > 40 then return end
    local key = tostring(node)
    if seen[key] then return end
    seen[key] = true
    local p = node:getProperties()
    if p.type == octane.NT_TEX_OSL or p.type == 117 then describe(node, path) end
    if p.isGraph then
        for _, item in ipairs(node:getOwnedItems()) do walk(item, path .. "/" .. tostring(item:getProperties().name), depth + 1) end
    end
    local okc, count = pcall(function() return node:getPinCount() end)
    for i = 1, (okc and count or 0) do
        local ok, c = pcall(function() return node:getInputNodeIx(i) end)
        if ok and c then walk(c, path .. "." .. tostring(node:getPinInfoIx(i).name), depth + 1) end
    end
end
try("walk", function()
    for _, item in ipairs(octane.project.getSceneGraph():getOwnedItems()) do
        walk(item, tostring(item:getProperties().name), 0)
    end
end)

pcall(function() octane.project.reset() end)
log("done " .. os.date())
local f = io.open(docs .. "orbis_probe.done", "w")
if f then f:write("done\n"); f:close() end
