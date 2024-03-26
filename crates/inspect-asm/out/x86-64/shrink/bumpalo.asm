inspect_asm::shrink::bumpalo:
	push r14
	push rbx
	push rax
	mov rbx, r9
	mov rax, r8
	neg rax
	cmp rdx, r8
	jae .LBB_1
	not rax
	xor ecx, ecx
	test rsi, rax
	cmovne rsi, rcx
	jmp .LBB_4
.LBB_1:
	mov rdx, qword ptr [rdi + 16]
	cmp qword ptr [rdx + 32], rsi
	jne .LBB_4
	mov r14, rcx
	sub r14, rbx
	and r14, rax
	inc rcx
	shr rcx
	cmp r14, rcx
	jb .LBB_4
	add r14, rsi
	mov qword ptr [rdx + 32], r14
	mov rdi, r14
	mov rdx, rbx
	call qword ptr [rip + memcpy@GOTPCREL]
	mov rsi, r14
.LBB_4:
	mov rax, rsi
	mov rdx, rbx
	add rsp, 8
	pop rbx
	pop r14
	ret