inspect_asm::alloc_u32_slice::try_bumpalo:
	push r15
	push r14
	push rbx
	mov rbx, rdx
	lea rdx, [4*rdx]
	mov rax, qword ptr [rdi + 16]
	mov r14, qword ptr [rax + 32]
	cmp rdx, r14
	ja .LBB_4
	sub r14, rdx
	and r14, -4
	cmp r14, qword ptr [rax]
	jb .LBB_4
	mov qword ptr [rax + 32], r14
.LBB_3:
	mov rdi, r14
	call qword ptr [rip + memcpy@GOTPCREL]
.LBB_6:
	mov rax, r14
	mov rdx, rbx
	pop rbx
	pop r14
	pop r15
	ret
.LBB_4:
	mov r15, rsi
	mov esi, 4
	mov r14, rdx
	call qword ptr [rip + bumpalo::Bump::alloc_layout_slow@GOTPCREL]
	mov rdx, r14
	mov rsi, r15
	mov r14, rax
	test rax, rax
	jne .LBB_3
	xor r14d, r14d
	jmp .LBB_6