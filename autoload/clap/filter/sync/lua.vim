" Author: liuchengxu <xuliuchengxlc@gmail.com>
" Description: Lua implementation of fzy filter algorithm.

function! clap#filter#sync#lua#(query, candidates, _winwidth, enable_icon, _line_splitter) abort
  let g:_clap_lua_query = a:query
  let g:_clap_lua_candidates = a:candidates
  let g:_clap_lua_enable_icon = a:enable_icon

lua << EOF
local fzy_filter = require('fzy_filter')
vim.g.__clap_fuzzy_matched_indices, vim.g._clap_lua_filtered =
    fzy_filter.do_fuzzy_match(vim.g._clap_lua_query, vim.g._clap_lua_candidates, vim.g._clap_lua_enable_icon)
EOF

  return g:_clap_lua_filtered
endfunction
