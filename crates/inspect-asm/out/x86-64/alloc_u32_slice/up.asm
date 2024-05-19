inspect_asm::alloc_u32_slice::up:
	push r15
	push r14
	push rbx
	mov rbx, rdx
	lea rdx, [4*rdx]
	mov rax, qword ptr [rdi]
	mov r14, qword ptr [rax]
	mov rcx, qword ptr [rax + 8]
	add r14, 3
	and r14, -4
	sub rcx, r14
	cmp rdx, rcx
	ja .LBB_2
	lea rcx, [r14 + rdx]
	mov qword ptr [rax], rcx
	test r14, r14
	je .LBB_2
.LBB_3:
	mov rdi, r14
	call qword ptr [rip + memcpy@GOTPCREL]
	mov rax, r14
	mov rdx, rbx
	pop rbx
	pop r14
	pop r15
	ret
.LBB_2:
	mov r14, rsi
	mov rsi, rbx
	mov r15, rdx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_slice_in_another_chunk
	mov rdx, r15
	mov rsi, r14
	mov r14, rax
	jmp .LBB_3