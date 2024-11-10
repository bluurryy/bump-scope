inspect_asm::vec_map::try_grow:
	push r15
	push r14
	push r13
	push r12
	push rbx
	mov rbx, rdi
	mov r14, qword ptr [rsi + 24]
	mov r12, qword ptr [rsi]
	mov r15, qword ptr [rsi + 8]
	mov r13, qword ptr [rsi + 16]
	test r15, r15
	je .LBB0_7
	mov rax, r15
	shr rax, 60
	jne .LBB0_5
	lea rcx, [8*r15]
	mov rdx, qword ptr [r14]
	mov rax, qword ptr [rdx]
	mov rsi, qword ptr [rdx + 8]
	add rax, 7
	and rax, -8
	sub rsi, rax
	cmp rcx, rsi
	ja .LBB0_0
	add rcx, rax
	mov qword ptr [rdx], rcx
	test rax, rax
	jne .LBB0_1
.LBB0_0:
	mov rdi, r14
	mov rsi, r15
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_slice_in_another_chunk
	test rax, rax
	je .LBB0_5
.LBB0_1:
	movabs rdx, 4611686018427387903
	lea rcx, [r15 - 1]
	mov rdi, rcx
	and rdi, rdx
	cmp rdi, rcx
	cmovae rdi, rcx
	cmp rdi, 18
	jbe .LBB0_2
	lea rsi, [r15 + rdx]
	and rsi, rdx
	cmp rsi, rcx
	cmovae rsi, rcx
	lea rdx, [r12 + 4*rsi]
	add rdx, 4
	cmp rax, rdx
	jae .LBB0_11
	lea rdx, [rax + 8*rsi]
	add rdx, 8
	cmp r12, rdx
	jae .LBB0_11
.LBB0_2:
	xor edx, edx
	mov rsi, r12
.LBB0_3:
	lea rdi, [r12 + 4*r15]
.LBB0_4:
	mov r8, rdx
	mov edx, dword ptr [rsi]
	mov qword ptr [rax + 8*r8], rdx
	lea rdx, [r8 + 1]
	cmp rcx, r8
	je .LBB0_8
	add rsi, 4
	cmp rsi, rdi
	jne .LBB0_4
	jmp .LBB0_8
.LBB0_5:
	lea rcx, [r12 + 4*r13]
	mov rax, qword ptr [r14]
	cmp rcx, qword ptr [rax]
	jne .LBB0_6
	mov qword ptr [rax], r12
.LBB0_6:
	mov qword ptr [rbx], 0
	jmp .LBB0_10
.LBB0_7:
	mov eax, 8
	xor edx, edx
.LBB0_8:
	lea rsi, [r12 + 4*r13]
	mov rcx, qword ptr [r14]
	cmp rsi, qword ptr [rcx]
	jne .LBB0_9
	mov qword ptr [rcx], r12
.LBB0_9:
	mov qword ptr [rbx], rax
	mov qword ptr [rbx + 8], rdx
	mov qword ptr [rbx + 16], rdx
	mov qword ptr [rbx + 24], r14
.LBB0_10:
	mov rax, rbx
	pop rbx
	pop r12
	pop r13
	pop r14
	pop r15
	ret
.LBB0_11:
	inc rdi
	movabs rdx, 9223372036854775804
	and rdx, rdi
	lea rsi, [r12 + 4*rdx]
	xor r8d, r8d
	xorps xmm0, xmm0
.LBB0_12:
	movsd xmm1, qword ptr [r12 + 4*r8]
	movsd xmm2, qword ptr [r12 + 4*r8 + 8]
	unpcklps xmm1, xmm0
	unpcklps xmm2, xmm0
	movups xmmword ptr [rax + 8*r8], xmm1
	movups xmmword ptr [rax + 8*r8 + 16], xmm2
	add r8, 4
	cmp rdx, r8
	jne .LBB0_12
	cmp rdi, rdx
	jne .LBB0_3
	jmp .LBB0_8
	lea rdx, [r12 + 4*r13]
	mov rcx, qword ptr [r14]
	cmp rdx, qword ptr [rcx]
	jne .LBB0_13
	mov qword ptr [rcx], r12
.LBB0_13:
	mov rdi, rax
	call _Unwind_Resume@PLT
