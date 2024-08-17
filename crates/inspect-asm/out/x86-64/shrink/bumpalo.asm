inspect_asm::shrink::bumpalo:
	push r14
	push rbx
	push rax
	mov rbx, r9
	cmp rdx, r8
	jae .LBB0_0
	dec r8
	xor eax, eax
	test r8, rsi
	cmovne rsi, rax
	jmp .LBB0_1
.LBB0_0:
	mov rax, qword ptr [rdi + 16]
	mov r14, qword ptr [rax + 32]
	cmp r14, rsi
	jne .LBB0_1
	mov rdx, rcx
	sub rdx, rbx
	neg r8
	and r8, rdx
	inc rcx
	shr rcx
	cmp r8, rcx
	jb .LBB0_1
	add r14, r8
	mov qword ptr [rax + 32], r14
	mov rdi, r14
	mov rdx, rbx
	call qword ptr [rip + memcpy@GOTPCREL]
	mov rsi, r14
.LBB0_1:
	mov rax, rsi
	mov rdx, rbx
	add rsp, 8
	pop rbx
	pop r14
	ret
