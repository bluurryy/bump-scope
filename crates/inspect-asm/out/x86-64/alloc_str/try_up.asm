inspect_asm::alloc_str::try_up:
	push r14
	push rbx
	push rax
	mov rbx, rdx
	mov rax, qword ptr [rdi]
	mov r14, qword ptr [rax]
	mov rcx, qword ptr [rax + 8]
	sub rcx, r14
	cmp rcx, rdx
	jb .LBB_3
	lea rcx, [r14 + rbx]
	mov qword ptr [rax], rcx
.LBB_2:
	mov rdi, r14
	mov rdx, rbx
	call qword ptr [rip + memcpy@GOTPCREL]
.LBB_5:
	mov rax, r14
	mov rdx, rbx
	add rsp, 8
	pop rbx
	pop r14
	ret
.LBB_3:
	mov r14, rsi
	mov rsi, rbx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_slice_in_another_chunk
	mov rsi, r14
	mov r14, rax
	test rax, rax
	jne .LBB_2
	xor r14d, r14d
	jmp .LBB_5