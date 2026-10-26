-- ORBIS probe 98 (unattended via scripts/dev/octane-run.sh): node pins for
-- glass (#9) — the absorption medium and the Universal material (its
-- transmission type enum) — name, type, default node, description and the
-- default nodes' attributes (enum values). The node-type constants with
-- MED / ABSORPTION / UNIVERSAL in their name are listed first.
-- Output: ~/Documents/orbis_probe98.txt, orbis_probe.done.

local home = os.getenv("HOME") or "/tmp"
local docs = home .. "/Documents/"
local lines = {}
local function log(s)
    lines[#lines + 1] = tostring(s)
    local f = io.open(docs .. "orbis_probe98.txt", "w")
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
try("constants", function()
    for name, v in pairs(octane) do
        if type(name) == "string" and (name:find("MED") or name:find("ABSORPTION") or name:find("UNIVERSAL")) and type(v) == "number" then
            log("const " .. name .. " = " .. v)
        end
    end
end)

local function describe(label, typ)
    try(label, function()
        local graph = octane.project.getSceneGraph()
        local node = octane.node.create{ type = typ, name = label, graphOwner = graph }
        log("== " .. label .. " type " .. tostring(typ) .. " pins " .. node:getPinCount())
        for i = 1, node:getPinCount() do
            local info = node:getPinInfoIx(i)
            log(string.format("pin %d %s", i, show(info)))
            local ok, c = pcall(function() return node:getInputNodeIx(i) end)
            if ok and c then
                local p = c:getProperties()
                local attrs = {}
                local okn, n = pcall(function() return c:getAttributeCount() end)
                for j = 1, (okn and n or 0) do
                    local ai = c:getAttributeInfoIx(j)
                    attrs[#attrs + 1] = show(ai) .. " value=" .. show(c:getAttributeIx(j))
                end
                log("   default node type " .. tostring(p.type) .. " " .. table.concat(attrs, " | "))
            end
        end
        local okn, n = pcall(function() return node:getAttributeCount() end)
        for j = 1, (okn and n or 0) do
            log("attr " .. show(node:getAttributeInfoIx(j)) .. " value=" .. show(node:getAttributeIx(j)))
        end
    end)
end

if octane.NT_MED_ABSORPTION then describe("absorption medium", octane.NT_MED_ABSORPTION) end
describe("universal", 130)

log("done " .. os.date())
local f = io.open(docs .. "orbis_probe.done", "w")
if f then f:write("done\n"); f:close() end
