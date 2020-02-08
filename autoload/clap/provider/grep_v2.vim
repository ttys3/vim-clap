" Author: liuchengxu <xuliuchengxlc@gmail.com>
" Description: Ivy-like file explorer.

scriptencoding utf-8

let s:save_cpo = &cpoptions
set cpoptions&vim

let s:grep = {}

function! s:handle_round_message(message) abort
  try
    let decoded = json_decode(a:message)
  catch
    call clap#helper#echo_error('Failed to decode message:'.a:message.', exception:'.v:exception)
    return
  endtry

  " Only process the latest request, drop the outdated responses.
  if s:last_request_id != decoded.id
    return
  endif

  if has_key(decoded, 'error')
    let error = decoded.error
    let s:grep_error_cache[error.dir] = error.message
    call g:clap.display.set_lines([error.message])
    call clap#indicator#set_matches('[??]')

  elseif has_key(decoded, 'result')
    let result = decoded.result

    if has_key(result, 'lines')
      call g:clap.display.set_lines(result.lines)
      call clap#sign#reset_to_first_line()
      call g:clap#display_win.shrink_if_undersize()
      return
    elseif result.total == 0
      call g:clap.display.set_lines(['EMPTY'])
    endif
    call clap#sign#reset_to_first_line()
    call clap#impl#refresh_matches_count(matchstr(string(result.total), '\d\+'))
    call g:clap#display_win.shrink_if_undersize()

  else
    call clap#helper#echo_error('This should not happen, neither error nor result is found.')
  endif
endfunction

function! s:send_message() abort
  let s:last_request_id += 1
  " Note: must use v:true/v:false for json_encode
  let msg = json_encode({
        \ 'method': 'grep',
        \ 'params': {'query': g:clap.input.get(), 'enable_icon': s:enable_icon},
        \ 'id': s:last_request_id
        \ })
  echom "sending:".msg
  call clap#rpc#send_message(msg)
endfunction

function! s:filter_or_send_message() abort
  call s:send_message()
endfunction

function! s:bs_action() abort
  call clap#highlight#clear()

  let input = g:clap.input.get()
  if input !=# ''
    call s:filter_or_send_message()
  endif
  return ''
endfunction

function! s:do_filter() abort
  let query = g:clap.input.get()
  echom 'do filter'
endfunction

function! s:reset_to(new_dir) abort
  call clap#highlight#clear()
  call g:clap.input.set('')
  call s:filter_or_send_message()
endfunction

function! s:tab_action() abort
  echom 'tab action'
 return ''
endfunction

function! s:grep_sink(selected) abort
  if g:clap_enable_icon
    let curline = a:selected[4:]
  else
    let curline = a:selected
  endif
  echom 'selected:'.a:selected
endfunction

function! s:grep_on_typed() abort
  call clap#rpc#stop()
  call s:start_rpc_service()
  " let s:last_request_id = 0
  " call clap#rpc#start(function('s:handle_round_message'))
  " call s:send_message()
  return ''
endfunction

function! s:grep_on_no_matches(input) abort
  execute 'edit' a:input
endfunction

function! s:start_rpc_service() abort
  let s:grep_cache = {}
  let s:grep_error_cache = {}
  let s:last_request_id = 0
  let s:enable_icon = g:clap_enable_icon ? v:true : v:false
  call clap#rpc#start(function('s:handle_round_message'))
  call s:send_message()
endfunction

let s:grep.init = function('s:start_rpc_service')
let s:grep.sink = function('s:grep_sink')
let s:grep.syntax = 'clap_grep'
let s:grep.on_typed = function('s:grep_on_typed')
let s:grep.source_type = g:__t_rpc
let s:grep.on_no_matches = function('s:grep_on_no_matches')
let g:clap#provider#grep_v2# = s:grep

let &cpoptions = s:save_cpo
unlet s:save_cpo
