local OSREPORT_STUB = 0x81300000

function read_cstring(emu, addr)
    local chars = {}
    for i = 0, 1023 do
        local b = emu:read_u8(addr + i)
        if b == 0 then break end
        chars[#chars + 1] = string.char(b)
    end
    return table.concat(chars)
end

function printf(emu, fmt, args)
    local out = {}
    local i = 1
    local arg_idx = 1
    local function next_arg()
        local v = args[arg_idx] or 0
        arg_idx = arg_idx + 1
        return v
    end

    while i <= #fmt do
        local c = fmt:sub(i, i)
        if c ~= "%" then
            out[#out + 1] = c
            i = i + 1
        else
            i = i + 1
            local spec = "%"
            if fmt:sub(i, i) == "0" then
                spec = spec .. "0"
                i = i + 1
            end
            while fmt:sub(i, i):match("%d") do
                spec = spec .. fmt:sub(i, i)
                i = i + 1
            end
            local kind = fmt:sub(i, i)
            i = i + 1
            if kind == "%" then
                out[#out + 1] = "%"
            elseif kind == "d" or kind == "i" then
                local v = next_arg()
                if v >= 0x80000000 then v = v - 0x100000000 end
                out[#out + 1] = string.format(spec .. "d", v)
            elseif kind == "u" then
                out[#out + 1] = string.format(spec .. "u", next_arg())
            elseif kind == "x" then
                out[#out + 1] = string.format(spec .. "x", next_arg())
            elseif kind == "X" then
                out[#out + 1] = string.format(spec .. "X", next_arg())
            elseif kind == "s" then
                out[#out + 1] = read_cstring(emu, next_arg())
            elseif kind == "c" then
                out[#out + 1] = string.char(next_arg() % 256)
            else
                out[#out + 1] = spec .. kind
            end
        end
    end
    return table.concat(out)
end

function emit(prefix, msg)
    msg = msg:gsub("[\r\n]+$", "")
    for line in (msg .. "\n"):gmatch("([^\n]*)\n") do
        if #line > 0 then
            log(prefix .. line)
        end
    end
end

function dispatch_printf(prefix)
    return function(emu)
        local fmt = read_cstring(emu, emu:gpr(3))
        local args = {
            emu:gpr(4), emu:gpr(5), emu:gpr(6), emu:gpr(7),
            emu:gpr(8), emu:gpr(9), emu:gpr(10),
        }
        emit(prefix, printf(emu, fmt, args))
    end
end

traps = traps or {}
traps.cpu_pre = traps.cpu_pre or {}
traps.cpu_pre[OSREPORT_STUB] = dispatch_printf("[apploader] ")
