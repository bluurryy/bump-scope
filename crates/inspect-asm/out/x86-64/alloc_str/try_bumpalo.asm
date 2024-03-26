inspect_asm::alloc_str::try_bumpalo:
	push r14
	push rbx
	push rax
	mov rbx, rdx
	mov rax, qword ptr [rdi + 16]
	mov r14, qword ptr [rax + 32]
	sub r14, rdx
	jb .LBB_4
	cmp r14, qword ptr [rax]
	jb .LBB_4
	mov qword ptr [rax + 32], r14
.LBB_3:
	mov rdi, r14
	mov rdx, rbx
	call qword ptr [rip + memcpy@GOTPCREL]
.LBB_6:
	mov rax, r14
	mov rdx, rbx
	add rsp, 8
	pop rbx
	pop r14
	ret
.LBB_4:
	mov r14, rsi
	mov esi, 1
	mov rdx, rbx
	call qword ptr [rip + bumpalo::Bump::alloc_layout_slow@GOTPCREL]
	mov rsi, r14
	mov r14, rax
	test rax, rax
	jne .LBB_3
	xor r14d, r14d
	jmp .LBB_6