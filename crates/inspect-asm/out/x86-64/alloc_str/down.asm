inspect_asm::alloc_str::down:
	push r14
	push rbx
	push rax
	mov r14, rdx
	mov rax, qword ptr [rdi]
	mov rbx, qword ptr [rax]
	mov rcx, rbx
	sub rcx, qword ptr [rax + 8]
	cmp rcx, rdx
	jb .LBB_2
	sub rbx, r14
	mov qword ptr [rax], rbx
.LBB_3:
	mov rdi, rbx
	mov rdx, r14
	call qword ptr [rip + memcpy@GOTPCREL]
	mov rax, rbx
	mov rdx, r14
	add rsp, 8
	pop rbx
	pop r14
	ret
.LBB_2:
	mov rbx, rsi
	mov rsi, r14
	call bump_scope::bump_scope::BumpScope<_,_,A>::do_alloc_slice_in_another_chunk
	mov rsi, rbx
	mov rbx, rax
	jmp .LBB_3