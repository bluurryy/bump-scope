inspect_asm::alloc_str::up_a:
	push r14
	push rbx
	push rax
	mov r14, rdx
	mov rax, qword ptr [rdi]
	mov rbx, qword ptr [rax]
	mov rcx, qword ptr [rax + 8]
	sub rcx, rbx
	cmp rcx, rdx
	jb .LBB_2
	lea rcx, [r14 + rbx]
	add rcx, 3
	and rcx, -4
	mov qword ptr [rax], rcx
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