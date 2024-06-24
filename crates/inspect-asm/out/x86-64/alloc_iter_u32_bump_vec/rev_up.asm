inspect_asm::alloc_iter_u32_bump_vec::rev_up:
	push rbp
	push r15
	push r14
	push r12
	push rbx
	sub rsp, 32
	mov qword ptr [rsp], 4
	xorps xmm0, xmm0
	movups xmmword ptr [rsp + 16], xmm0
	mov qword ptr [rsp + 8], rdi
	mov eax, 4
	test rdx, rdx
	jne .LBB0_1
	xor edx, edx
.LBB0_0:
	add rsp, 32
	pop rbx
	pop r12
	pop r14
	pop r15
	pop rbp
	ret
.LBB0_1:
	mov rbx, rdx
	mov r14, rsp
	mov rdi, r14
	mov r15, rsi
	mov rsi, rdx
	call bump_scope::mut_bump_vec_rev::MutBumpVecRev<T,A,_,_,_>::generic_grow_cold
	mov rax, r15
	mov rdx, qword ptr [rsp + 16]
	shl rbx, 2
	xor r12d, r12d
	jmp .LBB0_3
.LBB0_2:
	mov rcx, qword ptr [rsp]
	mov rsi, rdx
	not rsi
	mov dword ptr [rcx + 4*rsi], ebp
	inc rdx
	mov qword ptr [rsp + 16], rdx
	add r12, 4
	cmp rbx, r12
	je .LBB0_4
.LBB0_3:
	mov ebp, dword ptr [rax + r12]
	cmp qword ptr [rsp + 24], rdx
	jne .LBB0_2
	mov esi, 1
	mov rdi, r14
	call bump_scope::mut_bump_vec_rev::MutBumpVecRev<T,A,_,_,_>::generic_grow_cold
	mov rax, r15
	mov rdx, qword ptr [rsp + 16]
	jmp .LBB0_2
.LBB0_4:
	mov rax, qword ptr [rsp + 24]
	test rax, rax
	je .LBB0_5
	mov rbx, qword ptr [rsp + 8]
	lea rsi, [rcx + 4*rsi]
	shl rax, 2
	mov rdi, rcx
	sub rdi, rax
	lea r14, [rdi + 4*rdx]
	cmp r14, rsi
	jbe .LBB0_6
	mov rax, rsi
	mov r14, rcx
	jmp .LBB0_7
.LBB0_5:
	xor edx, edx
	mov eax, 4
	jmp .LBB0_0
.LBB0_6:
	lea rax, [4*rdx]
	mov r15, rdx
	mov rdx, rax
	mov r12, rdi
	call qword ptr [rip + memcpy@GOTPCREL]
	mov rdx, r15
	mov rax, r12
.LBB0_7:
	mov rcx, qword ptr [rbx]
	mov qword ptr [rcx], r14
	jmp .LBB0_0
