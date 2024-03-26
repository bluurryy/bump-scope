inspect_asm::alloc_str::try_down_a:
	push r14
	push rbx
	push rax
	mov rbx, rdx
	mov rax, qword ptr [rdi]
	mov r14, qword ptr [rax]
	mov rcx, r14
	sub rcx, qword ptr [rax + 8]
	cmp rcx, rdx
	jb .LBB_3
	sub r14, rbx
	and r14, -4
	mov qword ptr [rax], r14
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
	call bump_scope::bump_scope::BumpScope<A,_,_>::do_alloc_slice_in_another_chunk
	mov rsi, r14
	mov r14, rax
	test rax, rax
	jne .LBB_2
	xor r14d, r14d
	jmp .LBB_5