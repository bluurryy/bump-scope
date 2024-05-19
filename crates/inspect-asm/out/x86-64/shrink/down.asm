inspect_asm::shrink::down:
	push r15
	push r14
	push rbx
	mov rbx, r9
	lea rax, [r8 - 1]
	test rax, rsi
	jne .LBB_3
	mov rax, qword ptr [rdi]
	cmp qword ptr [rax], rsi
	je .LBB_4
	mov rbx, rcx
	mov rax, rsi
	jmp .LBB_8
.LBB_4:
	mov r14, rdi
	add rcx, rsi
	xor eax, eax
	cmp r8, 1
	mov edx, 0
	sbb rdx, r8
	sub rcx, rbx
	cmovb rcx, rax
	and rcx, rdx
	lea rax, [rsi + rbx]
	mov r15, rcx
	mov rdi, rcx
	mov rdx, rbx
	cmp rax, rcx
	jbe .LBB_5
	call qword ptr [rip + memmove@GOTPCREL]
	jmp .LBB_7
.LBB_5:
	call qword ptr [rip + memcpy@GOTPCREL]
.LBB_7:
	mov rcx, qword ptr [r14]
	mov rax, r15
	mov qword ptr [rcx], r15
.LBB_8:
	mov rdx, rbx
	pop rbx
	pop r14
	pop r15
	ret
.LBB_3:
	mov rdx, rcx
	mov rcx, r8
	mov r8, rbx
	call bump_scope::allocator::shrink::shrink_unfit
	mov rbx, rdx
	jmp .LBB_8